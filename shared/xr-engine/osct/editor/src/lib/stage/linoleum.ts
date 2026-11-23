import type { DataTexture } from "three";
import { random, repeatingTexture } from "./plywood";

/**
 * Blue-grey linoleum, drawn in code for the desk's linoleum top (#226; first Schöpfer's, #187): no image file, no
 * licence, a seeded sequence so every build is the same. A matt, even colour with a faint mottle (two scales of smooth
 * noise) and a faint speckle of lighter and darker grains (linoleum is pressed from linseed oil, cork and wood flour,
 * pigmented); it tiles both ways (the model's UVs: one repeat every 400 mm – `editor/tools/desk-eiermann.py`).
 */
export interface LinoleumOptions {
  /** The texture's size in pixels (square) */
  size?: number;
  seed?: number;
}

export interface LinoleumSheet {
  size: number;
  /** RGBA, 8 bits a channel, sRGB, row 0 at v = 0 */
  data: Uint8Array;
}

const BLUE_GREY: readonly [number, number, number] = [128, 156, 188];
/** The mottle's strength, the speckle's share of pixels (per thousand, lighter and darker) and their strength */
const MOTTLE = 0.03;
const LIGHT_GRAINS = 30;
const DARK_GRAINS = 20;
const GRAIN = 0.09;
const FINE = 0.012;

/** Smooth noise in -1..1 over a square of `size`, from a grid of `cells` × `cells` random values; it wraps (tiles) */
const valueNoise = (size: number, cells: number, next: () => number): Float32Array => {
  const grid = Float32Array.from({ length: cells * cells }, () => next() * 2 - 1);
  const at = (i: number, j: number) => grid[((j + cells) % cells) * cells + ((i + cells) % cells)];
  const ease = (t: number) => t * t * (3 - 2 * t);
  const out = new Float32Array(size * size);
  for (let y = 0; y < size; y++) {
    const gy = (y / size) * cells;
    const j = Math.floor(gy);
    const ty = ease(gy - j);
    for (let x = 0; x < size; x++) {
      const gx = (x / size) * cells;
      const i = Math.floor(gx);
      const tx = ease(gx - i);
      const top = at(i, j) + (at(i + 1, j) - at(i, j)) * tx;
      const bottom = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * tx;
      out[y * size + x] = top + (bottom - top) * ty;
    }
  }
  return out;
};

export function linoleum(options: LinoleumOptions = {}): LinoleumSheet {
  const size = Math.max(16, Math.round(options.size ?? 256));
  const next = random(options.seed ?? 23);
  const coarse = valueNoise(size, 6, next);
  const fine = valueNoise(size, 24, next);
  const data = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = y * size + x;
      let shade = 1 + MOTTLE * (0.65 * coarse[i] + 0.35 * fine[i]) + FINE * (next() * 2 - 1);
      const grain = Math.floor(next() * 1000);
      if (grain < LIGHT_GRAINS) shade *= 1 + GRAIN;
      else if (grain < LIGHT_GRAINS + DARK_GRAINS) shade *= 1 - GRAIN;
      for (let c = 0; c < 3; c++) data[i * 4 + c] = Math.round(Math.min(255, BLUE_GREY[c] * shade));
      data[i * 4 + 3] = 255;
    }
  }
  return { size, data };
}

/** The linoleum as a texture for the linoleum top's material `linoleum`: sRGB, repeating both ways, mipmapped */
export function linoleumTexture(options: LinoleumOptions = {}): DataTexture {
  const sheet = linoleum(options);
  return repeatingTexture("linoleum", sheet.size, sheet.size, sheet.data);
}
