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
  // snake drives on $11-$16; calc reads ASCII digits, ops, $3D, $43, $0D, $08
  // and accepts $15/$16 as +/-, so those two keys serve both apps
  ArrowUp: 0x11,
  ArrowDown: 0x12,
  ArrowLeft: 0x13,
  ArrowRight: 0x14,
  w: 0x11,
  s: 0x12,
  a: 0x13,
  d: 0x14,
  "+": 0x15, "-": 0x16, "_": 0x16,   // snake speed; calc accepts $15/$16 as +/-
  "=": 0x3D, "*": 0x2A, "/": 0x2F,
  "0": 0x30, "1": 0x31, "2": 0x32, "3": 0x33, "4": 0x34,
  "5": 0x35, "6": 0x36, "7": 0x37, "8": 0x38, "9": 0x39,
  c: 0x43, C: 0x43, Enter: 0x0D, Backspace: 0x08,
};

// Starter for the in-browser assembler: a live one-line edit of what the
// machine shows immediately.
const STARTER = `; VINTAGE-1 — edit me, then press assemble & run
        .org $E000
start:  lda #0
        sta $5804        ; phosphor: green
loop:   lda $5802        ; frame counter, low byte
        eor #$FF
        sta $4000        ; top scanline blinks with every frame
        jmp loop

        .org $FFFC
        .word start
`;

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
  vin_save_state(): number;
  vin_save_ptr(): number;
  vin_load_state(p: number, len: number): number;
  vin_asm(p: number, len: number): number;
  vin_asm_ptr(): number;
  vin_asm_err_ptr(): number;
  vin_asm_err_len(): number;
}


const WASM_URL = "/vintage.wasm";
// Phosphor presets for $5804, matching the headless PPM renderer in main.rs.
const PALETTES: { fg: [number, number, number]; bg: [number, number, number] }[] = [
  { fg: [51, 255, 51], bg: [6, 12, 6] },      // $5804 = 0: green
  { fg: [255, 176, 0], bg: [16, 12, 0] },     // $5804 = 1: amber
  { fg: [230, 230, 230], bg: [10, 10, 10] },  // $5804 = 2: white
];

// 2bpp color schemes for $5804 with bit 7 set, per fat-pixel index 0..3.
// Scheme 3 mixes the phosphor family; the ramps shade within one phosphor.
const SCHEMES: [number, number, number][][] = [
  [[6, 12, 6], [26, 102, 26], [51, 153, 51], [51, 255, 51]],
  [[16, 12, 0], [102, 61, 0], [153, 94, 0], [255, 176, 0]],
  [[10, 10, 10], [58, 58, 58], [120, 120, 120], [230, 230, 230]],
  [[6, 12, 6], [51, 255, 51], [255, 176, 0], [230, 230, 230]],
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

// Grow wasm linear memory until `base + bytes` fits, then refresh the heap
// view. Returns the base pointer for staging bytes before a handoff call.
function ensureHeap(bytes: number): number {
  const base = basePtr();
  const need = base + bytes - api.memory.buffer.byteLength;
  if (need > 0) api.memory.grow(Math.ceil(need / 65536));
  heap = new Uint8Array(api.memory.buffer);
  return base;
}

function loadRom(vin: Vin) {
  const total = (vin.banks.length + 1) * 0x2000;
  const base = ensureHeap(total);
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
  const pal = api.vin_palette();
  if (pal & 0x80) {
    // 2bpp: 4 fat pixels per byte, MSB pair leftmost; the machine's display
    // plane already holds the sprite-inverted indices.
    const scheme = SCHEMES[pal & 3];
    const px = fbImage.data;
    let di = 0;
    for (let i = 0; i < 0x1800; i++) {
      const b = fb[i];
      for (let p = 3; p >= 0; p--) {
        const c = scheme[(b >> (2 * p)) & 3];
        px[di++] = c[0];
        px[di++] = c[1];
        px[di++] = c[2];
        px[di++] = 255;
      }
    }
    ctxRef.putImageData(fbImage, 0, 0);
    return;
  }
  const { fg, bg } = PALETTES[pal % PALETTES.length];
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
    // Typing in the assembler textarea must not drive the machine.
    if ((e.target as HTMLElement).tagName === "TEXTAREA") return;
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
  (document.getElementById("asm-src") as HTMLTextAreaElement).value = STARTER;
  const demoNames = ["hello", "snake", "cube", "tune", "breakout", "banks", "calc"];
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

  // Save states: a .vst is a self-contained machine image — it includes the
  // cartridge banks, so it restores with no companion ROM needed.
  const save = () => {
    const n = api.vin_save_state();
    heap = new Uint8Array(api.memory.buffer); // fresh view: save allocated
    const p = api.vin_save_ptr();
    const bytes = heap.slice(p, p + n);
    const a = document.createElement("a");
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
    a.href = url;
    a.download = "vintage.vst";
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    (document.getElementById("vst-msg") as HTMLElement).textContent =
      `saved ${n.toLocaleString()} bytes`;
  };
  document.getElementById("save")!.addEventListener("click", save);
  const vstFile = document.getElementById("vst-file") as HTMLInputElement;
  document.getElementById("loadstate")!.addEventListener("click", () => vstFile.click());
  vstFile.addEventListener("change", async () => {
    const f = vstFile.files![0];
    if (!f) return;
    const vst = new Uint8Array(await f.arrayBuffer());
    const base = ensureHeap(vst.length);
    heap.set(vst, base);
    const ok = api.vin_load_state(base, vst.length);
    (document.getElementById("vst-msg") as HTMLElement).textContent =
      ok ? `state loaded (${vst.length.toLocaleString()} bytes)` : "not a .vst file";
  });

  // In-browser assembler: same two-pass assembler as the CLI, compiled into
  // the wasm core. Output flows straight through the regular V1/V1B loader.
  document.getElementById("asm-run")!.addEventListener("click", () => {
    const src = new TextEncoder().encode(
      (document.getElementById("asm-src") as HTMLTextAreaElement).value,
    );
    const base = ensureHeap(src.length);
    heap.set(src, base);
    const n = api.vin_asm(base, src.length);
    heap = new Uint8Array(api.memory.buffer); // fresh view: assembly allocates
    const msg = document.getElementById("asm-msg") as HTMLElement;
    if (n === 0) {
      msg.textContent = new TextDecoder().decode(
        new Uint8Array(heap.buffer as ArrayBuffer, api.vin_asm_err_ptr(), api.vin_asm_err_len()),
      );
      return;
    }
    const p = api.vin_asm_ptr();
    loadRom(parseVin(heap.slice(p, p + n).buffer));
    msg.textContent = `ok: ${n.toLocaleString()} bytes`;
  });
}

function markActive(btn: HTMLButtonElement) {
  document.querySelectorAll(".romBtn.active").forEach((x) => x.classList.remove("active"));
  btn.classList.add("active");
}

main();
