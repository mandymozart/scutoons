import { DataTexture, LinearFilter, LinearMipmapLinearFilter, RepeatWrapping, RGBAFormat, SRGBColorSpace, UnsignedByteType } from "three";

/**
 * The cut edge of a plywood board, drawn in code: no image file, no licence, a seeded sequence so every build is the
 * same. The texture is one stack of plies: u runs along the edge (the plies' length, it repeats seamlessly), v through
 * the board's thickness (v = 0 one face, v = 1 the other), so the plies lie parallel to the faces. The plies alternate
 * between birch veneers whose grain runs along the edge (light, long fine streaks) and cross-grain veneers seen end on
 * (a little darker, short marks across), the outer two always long-grain; thin glue lines part them, and half a glue
 * line lies on each outer edge, so stacks repeated in v meet in one whole line.
 */
export interface PlywoodOptions {
  /** The plies in one stack (at least 3; an odd count keeps both faces long-grain) */
  plies?: number;
  /** The texture's size in pixels: along the edge and through the thickness */
  width?: number;
  height?: number;
  seed?: number;
}

export interface Ply {
  /** The ply's rows (pixels from v = 0, before the boundaries' waviness) */
  from: number;
  to: number;
  /** A cross-grain veneer (seen end on) rather than one whose grain runs along the edge */
  cross: boolean;
}

export interface PlywoodEdge {
  width: number;
  height: number;
  /** RGBA, 8 bits a channel, sRGB, row 0 at v = 0 */
  data: Uint8Array;
  plies: Ply[];
}

const LONG: readonly [number, number, number] = [226, 211, 182];
const CROSS: readonly [number, number, number] = [197, 172, 137];
const GLUE: readonly [number, number, number] = [92, 68, 46];
/** How dark a glue line is at its middle, and its half width in pixels */
const GLUE_STRENGTH = 0.65;
const GLUE_HALF_WIDTH = 0.9;
/** How far the plies' boundaries wander through the thickness (pixels) and how much the plies' thickness varies */
const WAVE = 0.6;
const THICKNESS_JITTER = 0.12;

/** A seeded sequence of numbers in 0..1 (the drawn textures of the desks: `plywood.ts`, `oak.ts`, `linoleum.ts`) */
export const random = (seed: number) => {
  let state = seed >>> 0 || 1;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

/** Noise in -1..1 along a line of `count`, smoothed over `radius` neighbours and wrapping (so it tiles) */
export const smoothNoise = (count: number, radius: number, next: () => number): Float32Array => {
  const raw = Float32Array.from({ length: count }, () => next() * 2 - 1);
  const out = new Float32Array(count);
  let peak = 0;
  for (let i = 0; i < count; i++) {
    let sum = 0;
    for (let k = -radius; k <= radius; k++) sum += raw[(i + k + count) % count];
    out[i] = sum / (2 * radius + 1);
    peak = Math.max(peak, Math.abs(out[i]));
  }
  return peak > 0 ? out.map(value => value / peak) : out;
};

/** The pixels of one stack of plies (see the module's comment) */
export function plywoodEdge(options: PlywoodOptions = {}): PlywoodEdge {
  const count = Math.max(3, Math.round(options.plies ?? 7));
  const width = Math.max(8, Math.round(options.width ?? 512));
  const height = Math.max(count * 4, Math.round(options.height ?? 128));
  const next = random(options.seed ?? 1);

  // The plies' thickness: equal, give or take a little
  const weights = Array.from({ length: count }, () => 1 + (next() * 2 - 1) * THICKNESS_JITTER);
  const total = weights.reduce((sum, w) => sum + w, 0);
  const bounds = [0];
  weights.forEach(w => bounds.push(bounds[bounds.length - 1] + (w / total) * height));
  bounds[count] = height;
  const plies: Ply[] = weights.map((_, i) => ({ from: bounds[i], to: bounds[i + 1], cross: i % 2 === 1 }));
  // Each inner boundary wanders a little along the edge (whole waves, so u still tiles)
  const waves = bounds.map((_, i) => (i === 0 || i === count ? null : { k: 1 + Math.floor(next() * 3), phase: next() * Math.PI * 2, amp: WAVE * (0.5 + next() * 0.5) }));
  const tones = plies.map(() => 1 + (next() * 2 - 1) * 0.03);

  // The grain: long fibres are streaks along u (a value per row, drifting slowly along the edge); cross plies show
  // their fibres end on, as short marks across (a value per column) with a few pores
  const rowGrain = smoothNoise(height, 1, next);
  const rowDrift = Array.from({ length: height }, () => {
    const phase = next() * Math.PI * 2;
    return { k: Math.floor(next() * 4), sin: Math.sin(phase), cos: Math.cos(phase) };
  });
  // sin(a + phase) = sin a · cos phase + cos a · sin phase, with sin a and cos a tabled once per wave count (1–4)
  const waveSin = [1, 2, 3, 4].map(k => Float32Array.from({ length: width }, (_, x) => Math.sin((2 * Math.PI * k * x) / width)));
  const waveCos = [1, 2, 3, 4].map(k => Float32Array.from({ length: width }, (_, x) => Math.cos((2 * Math.PI * k * x) / width)));
  const columnGrain = smoothNoise(width, 1, next);
  const columnCoarse = smoothNoise(width, 6, next);

  const data = new Uint8Array(width * height * 4);
  const boundaryAt = (i: number, x: number) => {
    const wave = waves[i];
    return wave ? bounds[i] + wave.amp * Math.sin((2 * Math.PI * wave.k * x) / width + wave.phase) : bounds[i];
  };
  for (let x = 0; x < width; x++) {
    const edges = bounds.map((_, i) => boundaryAt(i, x));
    let ply = 0;
    for (let y = 0; y < height; y++) {
      const v = y + 0.5;
      while (ply < count - 1 && v >= edges[ply + 1]) ply++;
      const cross = plies[ply].cross;
      const base = cross ? CROSS : LONG;
      let shade = tones[ply];
      if (cross) {
        shade *= 1 + 0.07 * columnGrain[x] + 0.04 * columnCoarse[x];
        const h = Math.imul(x * 73856093 ^ y * 19349663, 0x9e3779b1) >>> 0;
        if (h % 1000 < 18) shade *= 0.86;
      } else {
        const drift = rowDrift[y];
        shade *= 1 + 0.05 * rowGrain[y] + 0.025 * (waveSin[drift.k][x] * drift.cos + waveCos[drift.k][x] * drift.sin);
      }
      // The nearest glue line: the ply's own boundaries, inner ones or half of one on either outer edge
      const distance = Math.min(v - edges[ply], Math.abs(edges[ply + 1] - v));
      const glue = GLUE_STRENGTH * Math.max(0, 1 - distance / (GLUE_HALF_WIDTH + 0.5));
      const offset = (y * width + x) * 4;
      for (let c = 0; c < 3; c++) {
        const wood = Math.min(255, base[c] * shade);
        data[offset + c] = Math.round(wood + (GLUE[c] - wood) * glue);
      }
      data[offset + 3] = 255;
    }
  }
  return { width, height, data, plies };
}

/** The plies as a texture for the edges' material: sRGB, repeating both ways, mipmapped */
export function plywoodEdgeTexture(options: PlywoodOptions = {}): DataTexture {
  const edge = plywoodEdge(options);
  return repeatingTexture("plywood-edge", edge.width, edge.height, edge.data);
}

/** Drawn RGBA pixels (sRGB, row 0 at v = 0) as a texture: repeating both ways, mipmapped */
export function repeatingTexture(name: string, width: number, height: number, data: Uint8Array): DataTexture {
  const texture = new DataTexture(data, width, height, RGBAFormat, UnsignedByteType);
  texture.name = name;
  texture.colorSpace = SRGBColorSpace;
  texture.wrapS = texture.wrapT = RepeatWrapping;
  texture.magFilter = LinearFilter;
  texture.minFilter = LinearMipmapLinearFilter;
  texture.generateMipmaps = true;
  texture.anisotropy = 8;
  texture.needsUpdate = true;
  return texture;
}
