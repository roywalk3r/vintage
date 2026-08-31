// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! VINTAGE-1 console front-end: loads the wasm core, parses .vin ROMs,
//! drives run_frame per animation tick, blits the 1-bit framebuffer.

type Vin = { rom: Uint8Array; banks: Uint8Array[] };

const W = 256;
const H = 192;

function parseVin(buf: ArrayBuffer): Vin {
  const d = new DataView(buf);
  const magic = String.fromCharCode(d.getUint8(0), d.getUint8(1));
  if (magic !== "V1") throw new Error("bad magic");
  // V1B shares the "V1" prefix; byte 2 == 'B' switches to the banked layout.
  const banked = d.getUint8(2) === 0x42;
  const banks: Uint8Array[] = [];
  let off = banked ? 5 : 4;
  const nbanks = banked ? d.getUint16(3, true) : 1;
  for (let b = 0; b < nbanks; b++) {
    const nseg = banked ? d.getUint16(off, true) : d.getUint16(2, true);
    const rom = new Uint8Array(0x2000);
    for (let i = 0; i < nseg; i++) {
      const addr = d.getUint16(off, true);
      const len = d.getUint16(off + 2, true);
      off += 4;
      rom.set(new Uint8Array(buf, off, len), addr - 0xe000);
      off += len;
    }
    banks.push(rom);
  }
  return { rom: banks.shift()!, banks };
}

const KEYMAP: Record<string, number> = {
  ArrowUp: 0x11, ArrowDown: 0x12, ArrowLeft: 0x13, ArrowRight: 0x14,
  w: 0x11, s: 0x12, a: 0x13, d: 0x14,
  "+": 0x15, "=": 0x15, "-": 0x16, "_": 0x16,
};

interface VintageApi {
  memory: WebAssembly.Memory;
  vin_reset(): void;
  vin_load_banks(p: number, n: number): void;
  vin_cpu_reset(): void;
  vin_step(): number;
  vin_run_frame(): void;
  vin_fb_ptr(): number;
  vin_key(c: number): void;
  vin_beeper(): number;
  vin_palette(): number;
  vin_rd(a: number): number;
  vin_wr(a: number, v: number): void;
  vin_cpu_state_ptr(): number;
  vin_cycles(): number;
}


const WASM_URL = "/vintage.wasm";
// Phosphor presets for $5804, matching the headless PPM renderer in main.rs.
const PALETTES: { fg: [number, number, number]; bg: [number, number, number] }[] = [
  { fg: [51, 255, 51], bg: [6, 12, 6] },      // $5804 = 0: green
  { fg: [255, 176, 0], bg: [16, 12, 0] },     // $5804 = 1: amber
  { fg: [230, 230, 230], bg: [10, 10, 10] },  // $5804 = 2: white
];

let api: VintageApi;
let heap: Uint8Array;
let fbImage: ImageData;
let running = true;
let budgetPct = 100;

// Per-demo cycle accounting: 32-frame rolling average of cycles/frame,
// plus the total since this ROM booted.
const CYC_WINDOW = 32;
let lastCycles = 0;
const cycWindow = new Float64Array(CYC_WINDOW);
let cycIdx = 0;

async function main() {
  const { instance } = await WebAssembly.instantiateStreaming(
    fetch(WASM_URL), {},
  );
  api = instance.exports as unknown as VintageApi & { memory: WebAssembly.Memory };
  heap = new Uint8Array(api.memory.buffer);

  const canvas = document.getElementById("screen") as HTMLCanvasElement;
  const ctx = canvas.getContext("2d")!;
  fbImage = ctx.createImageData(W, H);

  const resp = await fetch("/demos/cube.vin");
  loadRom(parseVin(await resp.arrayBuffer()));
  bindUi(ctx);
  requestAnimationFrame(tick);
}

function loadRom(vin: Vin) {
  const base = basePtr();
  const total = (vin.banks.length + 1) * 0x2000;
  if (base + total > api.memory.buffer.byteLength) {
    api.memory.grow(1);
  }
  heap = new Uint8Array(api.memory.buffer);
  const flat = new Uint8Array(total);
  flat.set(vin.rom, 0);
  vin.banks.forEach((b, i) => flat.set(b, (i + 1) * 0x2000));
  heap.set(flat, base);
  api.vin_load_banks(base, vin.banks.length + 1);
  api.vin_cpu_reset();
  lastCycles = 0;
  cycWindow.fill(0);
  cycIdx = 0;
}

function basePtr(): number {
  const ex = api as unknown as { __heap_base: WebAssembly.Global };
  return ex.__heap_base.value;
}

function tick() {
  if (running && budgetPct === 100) {
    api.vin_run_frame();
  } else if (running) {
    for (let i = 0; i < budgetPct * 333; i++) api.vin_step();
  }
  audioFrame();
  blit();
  cpuPanel();
  cycPanel();
  requestAnimationFrame(tick);
}

function cycPanel() {
  // u64 crosses the C ABI as a BigInt; JS numbers are plenty for displays.
  const now = Number(api.vin_cycles());
  const delta = now - lastCycles;
  lastCycles = now;
  // Frames where the machine is paused report delta 0; counting them
  // would drag the average down, so only running frames enter the window.
  if (delta > 0) {
    cycWindow[cycIdx++ % CYC_WINDOW] = delta;
  }
  let avg = 0;
  for (const v of cycWindow) avg += v;
  avg /= CYC_WINDOW;
  const el = document.getElementById("cyc")!;
  el.textContent =
    `cycles=$${now.toString(16).toUpperCase()}` +
    `\nlast frame=${delta.toLocaleString()} cyc` +
    `\nper frame ≈ ${avg === 0 ? "—" : Math.round(avg).toLocaleString()} cyc`;
}

let ctxRef: CanvasRenderingContext2D;

// The beeper is one square-wave channel: $5807 holds the half-period in
// CPU cycles, 0 silences. Browsers require a user gesture before audio,
// so the context is created lazily on the first keypress or click.
let audioCtx: AudioContext | null = null;
let audioOsc: OscillatorNode | null = null;
let audioGain: GainNode | null = null;

function ensureAudio() {
  if (audioCtx) return;
  audioCtx = new AudioContext();
  audioOsc = audioCtx.createOscillator();
  audioOsc.type = "square";
  audioGain = audioCtx.createGain();
  audioGain.gain.value = 0;
  audioOsc.connect(audioGain).connect(audioCtx.destination);
  audioOsc.start();
}

const BEEPER_CLOCK = 120_000; // effective toggle clock, Hz

function audioFrame() {
  if (!audioCtx) return;
  const n = api.vin_beeper();
  if (n === 0) {
    audioGain!.gain.value = 0;
  } else {
    audioOsc!.frequency.value = BEEPER_CLOCK / (2 * n);
    audioGain!.gain.value = 0.04;
  }
}

function blit() {
  const fb = new Uint8Array(api.memory.buffer, api.vin_fb_ptr(), 0x1800);
  const { fg, bg } = PALETTES[api.vin_palette() % PALETTES.length];
  const px = fbImage.data;
  let di = 0;
  for (let i = 0; i < 0x1800; i++) {
    const b = fb[i];
    for (let bit = 7; bit >= 0; bit--) {
      const on = (b >> bit) & 1;
      const c = on ? fg : bg;
      px[di++] = c[0];
      px[di++] = c[1];
      px[di++] = c[2];
      px[di++] = 255;
    }
  }
  ctxRef.putImageData(fbImage, 0, 0);
}

function cpuPanel() {
  const st = new Uint8Array(api.memory.buffer, api.vin_cpu_state_ptr(), 8);
  const f = (v: number) => v.toString(16).padStart(2, "0").toUpperCase();
  const fl = st[6];
  const names = "NV-BDIZC";
  let flags = "";
  for (let i = 0; i < 8; i++) flags += (fl >> (7 - i)) & 1 ? names[i] : ".";
  const el = document.getElementById("cpu")!;
  el.textContent =
    `A=${f(st[0])} X=${f(st[1])} Y=${f(st[2])} SP=${f(st[3])}\n` +
    `PC=$${f(st[5])}${f(st[4])} P=${f(fl)} [${flags}]\n` +
    `cycles LSB=$${f(st[7])}`;
}

function bindUi(ctx: CanvasRenderingContext2D) {
  ctxRef = ctx;
  window.addEventListener("keydown", (e) => {
    ensureAudio();
    const code = KEYMAP[e.key];
    if (code !== undefined) {
      e.preventDefault();
      api.vin_key(code);
    }
  });
  window.addEventListener("keyup", (e) => {
    if (KEYMAP[e.key] !== undefined) e.preventDefault();
  });
  const runBtn = document.getElementById("run")!;
  runBtn.addEventListener("click", () => {
    ensureAudio();
    running = !running;
    runBtn.textContent = running ? "pause" : "resume";
  });
  document.getElementById("step")!.addEventListener("click", () => {
    running = false;
    (document.getElementById("run") as HTMLButtonElement).textContent = "resume";
    api.vin_step();
  });
  document.getElementById("frame")!.addEventListener("click", () => {
    running = false;
    (document.getElementById("run") as HTMLButtonElement).textContent = "resume";
    api.vin_run_frame();
  });
  const slider = document.getElementById("budget") as HTMLInputElement;
  slider.addEventListener("input", () => {
    budgetPct = Number(slider.value);
    const e = document.getElementById("hint")!;
    e.textContent = budgetPct === 100 ? "100% = full speed"
      : `${budgetPct}% ≈ ${budgetPct * 333} cycles/frame`;
  });
  const demoNames = ["hello", "snake", "cube", "tune", "breakout"];
  const romList = document.getElementById("roms")!;
  for (const n of demoNames) {
    const b = document.createElement("button");
    b.className = "romBtn";
    b.textContent = n;
    b.addEventListener("click", async () => {
      const r = await fetch(`/demos/${n}.vin`);
      loadRom(parseVin(await r.arrayBuffer()));
      markActive(b);
    });
    romList.appendChild(b);
  }
  const fileInput = document.getElementById("file") as HTMLInputElement;
  document.getElementById("file-well")!.addEventListener("click", () => fileInput.click());
  fileInput.addEventListener("change", async () => {
    const f = fileInput.files![0];
    if (!f) return;
    loadRom(parseVin(await f.arrayBuffer()));
  });
}

function markActive(btn: HTMLButtonElement) {
  document.querySelectorAll(".romBtn.active").forEach((x) => x.classList.remove("active"));
  btn.classList.add("active");
}

main();
