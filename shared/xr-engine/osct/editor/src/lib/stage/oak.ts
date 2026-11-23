import type { DataTexture } from "three";
import { random, repeatingTexture, smoothNoise } from "./plywood";

/**
 * Light oak, drawn in code for the linoleum top's oak-veneered underside (#226; first Schöpfer's desk's, #187): no
 * image file, no licence, a seeded sequence so every build is the same. u runs along the grain, v across it, and the
 * texture tiles both ways (the model's UVs: one repeat every 500 mm along the grain, 125 mm across –
 * `editor/tools/desk-eiermann.py`). The grain is a run of fine, slightly wavy latewood lines of varying strength along
 * u, with faint streaks between them and the short dark dashes of oak's open pores.
 */
export interface OakOptions {
  /** The texture's size in pixels: along the grain and across it */
  width?: number;
  height?: number;
  seed?: number;
}

export interface OakGrain {
  width: number;
  height: number;
  /** RGBA, 8 bits a channel, sRGB, row 0 at v = 0 */
  data: Uint8Array;
}

/** The wood between the lines (a light oiled oak) and the latewood lines */
const EARLY: readonly [number, number, number] = [186, 162, 130];
const LATE: readonly [number, number, number] = [128, 100, 72];
/** Grain lines across the texture's height (a whole number, so v tiles): about one every 5 mm at 125 mm */
const LINES = 24;
/** How far the lines wander across the grain (pixels), how wide a line is (a fraction of the spacing), how dark */
const WAVE = 2.5;
const LINE_WIDTH = 0.16;
const LINE_STRENGTH = 0.55;
/** The pores: how many dashes per thousand pixels, their length (pixels) and how dark */
const PORES = 22;
const PORE_LENGTH = 7;
const PORE_DARKEN = 0.86;

export function oakGrain(options: OakOptions = {}): OakGrain {
  const width = Math.max(16, Math.round(options.width ?? 512));
  const height = Math.max(LINES * 2, Math.round(options.height ?? 128));
  const next = random(options.seed ?? 11);

  // The lines wander with a few whole waves along u (so u tiles), each line a strength of its own
  const waves = [1, 2, 3].map(k => ({ k, amp: (WAVE / k) * (0.5 + next() * 0.5), phase: next() * Math.PI * 2 }));
  const wander = Float32Array.from({ length: width }, (_, x) => waves.reduce((sum, w) => sum + w.amp * Math.sin((2 * Math.PI * w.k * x) / width + w.phase), 0));
  const strength = Array.from({ length: LINES }, () => 0.35 + next() * 0.65);
  // Across the grain: the lines' spacing varies a little, and fine streaks lie between them (both wrap, so v tiles)
  const jitter = smoothNoise(height, 4, next);
  const streaks = smoothNoise(height, 1, next);
  const tone = smoothNoise(width, 24, next);
  const poreShift = Array.from({ length: height }, () => Math.floor(next() * PORE_LENGTH));

  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const v = (((y + wander[x]) % height) + height) % height;
      const row = Math.floor(v);
      const phase = (LINES * v) / height + 0.18 * jitter[row];
      const fraction = phase - Math.floor(phase);
      const distance = Math.min(fraction, 1 - fraction);
      const line = Math.max(0, 1 - distance / LINE_WIDTH) ** 2 * strength[((Math.round(phase) % LINES) + LINES) % LINES] * LINE_STRENGTH;
      let shade = (1 + 0.035 * streaks[row]) * (1 + 0.03 * tone[x]);
      const dash = Math.floor((x + poreShift[y]) / PORE_LENGTH);
      const h = Math.imul(dash * 73856093 ^ y * 19349663, 0x9e3779b1) >>> 0;
      if (h % 1000 < PORES) shade *= PORE_DARKEN;
      const offset = (y * width + x) * 4;
      for (let c = 0; c < 3; c++) data[offset + c] = Math.round(Math.min(255, (EARLY[c] + (LATE[c] - EARLY[c]) * line) * shade));
      data[offset + 3] = 255;
    }
  }
  return { width, height, data };
}

/** The oak as a texture for the linoleum top's material `oak`: sRGB, repeating both ways, mipmapped */
export function oakTexture(options: OakOptions = {}): DataTexture {
  const oak = oakGrain(options);
  return repeatingTexture("oak", oak.width, oak.height, oak.data);
}
