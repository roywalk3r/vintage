// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! VINTAGE-1 console front-end: loads the wasm core, parses .vin ROMs,
//! drives run_frame per animation tick, blits the 1-bit framebuffer.

type Vin = { rom: Uint8Array };

const W = 256;
const H = 192;

function parseVin(buf: ArrayBuffer): Vin {
  const d = new DataView(buf);
  const magic = String.fromCharCode(d.getUint8(0), d.getUint8(1));
  if (magic !== "V1") throw new Error("bad magic");
  const nseg = d.getUint16(2, true);
  const rom = new Uint8Array(0x2000);
  let off = 4;
  for (let i = 0; i < nseg; i++) {
    const addr = d.getUint16(off, true);
    const len = d.getUint16(off + 2, true);
    off += 4;
    rom.set(new Uint8Array(buf, off, len), addr - 0xe000);
    off += len;
  }
  return { rom };
}

const KEYMAP: Record<string, number> = {
  ArrowUp: 0x11, ArrowDown: 0x12, ArrowLeft: 0x13, ArrowRight: 0x14,
  w: 0x11, s: 0x12, a: 0x13, d: 0x14,
  "+": 0x15, "=": 0x15, "-": 0x16, "_": 0x16,
};

interface VintageApi {
  memory: WebAssembly.Memory;
  vin_reset(): void;
  vin_load_rom(p: number, n: number): void;
  vin_cpu_reset(): void;
  vin_step(): number;
  vin_run_frame(): void;
  vin_fb_ptr(): number;
  vin_key(c: number): void;
  vin_rd(a: number): number;
  vin_wr(a: number, v: number): void;
  vin_cpu_state_ptr(): number;
}


const WASM_URL = "/vintage.wasm";
const FG = [83, 255, 126];
const BG = [4, 14, 6];

let api: VintageApi;
let heap: Uint8Array;
let fbImage: ImageData;
let running = true;
let budgetPct = 100;

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
  if (base + vin.rom.length > api.memory.buffer.byteLength) {
    api.memory.grow(1);
  }
  heap = new Uint8Array(api.memory.buffer);
  heap.set(vin.rom, base);
  api.vin_load_rom(base, 0x2000);
  api.vin_cpu_reset();
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
  blit();
  cpuPanel();
  requestAnimationFrame(tick);
}

let ctxRef: CanvasRenderingContext2D;

function blit() {
  const fb = new Uint8Array(api.memory.buffer, api.vin_fb_ptr(), 0x1800);
  const px = fbImage.data;
  let di = 0;
  for (let i = 0; i < 0x1800; i++) {
    const b = fb[i];
    for (let bit = 7; bit >= 0; bit--) {
      const on = (b >> bit) & 1;
      const c = on ? FG : BG;
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
  const demoNames = ["hello", "snake", "cube"];
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
