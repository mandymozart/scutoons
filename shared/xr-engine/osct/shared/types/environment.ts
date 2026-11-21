/**
 * The experience's environment (content schema 2.8, asset references 2.10): the world around the reader while scanning and
 * the onion sky (`engine/src/scene/look-around`, docs/look-around.md). Experience-level only – no per-scene overrides yet.
 * The reader's Graphics switches (Info → Settings) still apply on top: the content says what is drawn, the reader can
 * turn either off. An experience without `environment` keeps today's look (`DEFAULT_ENVIRONMENT`).
 *
 * The world and the sky can be **assets** (2.10): `worldSrc` names a glTF model (the scenery around the reader, in
 * place of the built-in world's), `skySrc` an equirectangular image (the sky, in place of the built-in world's). Each
 * holds a content path (`/assets/content/…`, the content build writes it from a path below the content folder) or an
 * absolute URL (an uploaded asset of the editor's library). Without them the built-in `world` is drawn whole.
 */

import { isObject } from "../guards/primitives";

export const ENVIRONMENT_WORLDS = ["cafe"] as const;
export type EnvironmentWorld = (typeof ENVIRONMENT_WORLDS)[number];

/** The files a world model may be (`worldSrc`) and a sky may be (`skySrc`), by extension */
export const WORLD_MODEL_EXTENSIONS = [".glb", ".gltf"] as const;
export const SKY_IMAGE_EXTENSIONS = [".jpg", ".jpeg", ".png", ".webp"] as const;

export interface EnvironmentData {
  /** The built-in world: drawn whole without assets, and the fallback while an asset loads or when it fails */
  world: EnvironmentWorld;
  /** Looking up, the sky in the camera picture shows the world's sky */
  onionSky: boolean;
  /**
   * Optional (2.10): the scenery around the reader, a glTF model – its origin is the floor under the reader, its +y up,
   * its −z towards the scene; the reader's eye is `WORLD_EYE_HEIGHT` above the origin. Seen from that point only (3DoF).
   */
  worldSrc?: string;
  /** Optional (2.10): the sky, an equirectangular image (2:1, its top edge straight up, its centre towards the scene) */
  skySrc?: string;
}

export const DEFAULT_ENVIRONMENT: EnvironmentData = { world: "cafe", onionSky: true };

/** Metres from a world model's origin (the floor) up to the reader's eye – the built-in world's floor lies as deep */
export const WORLD_EYE_HEIGHT = 1.6;

/** The fields of an environment; older apps' guards (2.8) refuse others, this one lets unknown fields pass */
const FIELDS = ["world", "onionSky", "worldSrc", "skySrc"] as const;

/** The experience's environment, the default when it has none */
export const environmentOf = (experience: { environment?: EnvironmentData } | null | undefined): EnvironmentData =>
  experience?.environment ?? DEFAULT_ENVIRONMENT;

/** The extension of a reference's path (lower case, without query or fragment), "" when it has none */
export const referenceExtension = (reference: string): string => {
  const path = reference.split(/[?#]/)[0];
  const dot = path.lastIndexOf(".");
  return dot > path.lastIndexOf("/") ? path.slice(dot).toLowerCase() : "";
};

/** A content path or an absolute http(s) URL ending in one of the extensions */
const referenceProblem = (value: unknown, path: string, what: string, extensions: readonly string[]): string | null => {
  if (typeof value !== "string" || !(value.startsWith("/assets/content/") || /^https?:\/\/[^/]+\//.test(value))) {
    return `${path} must be a content path (/assets/content/…) or an absolute URL of ${what}.`;
  }
  return extensions.includes(referenceExtension(value)) ? null : `${path} must be ${what} (${extensions.join(", ")}).`;
};

/**
 * The problems of an `environment` value; [] when valid. `strict` (the content build and the editor API) also refuses
 * fields the contract does not know; the reader's guard lets them pass, so a later MINOR's additions load.
 */
export const environmentProblems = (value: unknown, path = "environment", strict = true): string[] => {
  if (!isObject(value)) return [`${path} must be an object { world, onionSky }.`];
  const { world, onionSky, worldSrc, skySrc } = value;
  const problems: string[] = [];
  if (!(ENVIRONMENT_WORLDS as readonly unknown[]).includes(world)) problems.push(`${path}.world must be one of ${ENVIRONMENT_WORLDS.join(", ")}.`);
  if (typeof onionSky !== "boolean") problems.push(`${path}.onionSky must be true or false.`);
  if (worldSrc !== undefined) {
    const problem = referenceProblem(worldSrc, `${path}.worldSrc`, "a glTF model", WORLD_MODEL_EXTENSIONS);
    if (problem) problems.push(problem);
  }
  if (skySrc !== undefined) {
    const problem = referenceProblem(skySrc, `${path}.skySrc`, "an equirectangular image", SKY_IMAGE_EXTENSIONS);
    if (problem) problems.push(problem);
  }
  if (strict) {
    for (const key of Object.keys(value)) {
      if (!(FIELDS as readonly string[]).includes(key)) problems.push(`${path}.${key} is not a field of the environment.`);
    }
  }
  return problems;
};
