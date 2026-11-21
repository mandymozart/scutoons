/**
 * A model's colour map in several sizes (content schema 2.27, #260): the mesh is loaded once, the map by the reader's
 * texture quality. The model file carries the smallest map, so it shows alone – in an older app, and at once in this
 * one; the app then loads the size the quality asks for and puts it in place of the file's map.
 *
 * A set belongs to one model asset (`AssetData.textures`). `material` names the glTF material whose map it is; none =
 * every material of the model that has a map (a model baked into one map – the Blender batch's output,
 * docs/characters.md). The sizes are the whole map at 512, 1024, 2048 or 4096 px, each once.
 */

import { isObject } from "../guards/primitives";

/** The sizes a texture set may hold (px, square maps): what the reader's texture quality chooses from */
export const TEXTURE_SIZES = [512, 1024, 2048, 4096] as const;
export type TextureSize = (typeof TEXTURE_SIZES)[number];

/** One size of a map: the file with the whole map at `size` px */
export interface TextureSizeData {
  size: TextureSize;
  src: string;
}

export interface TextureSetData {
  material?: string;
  /** At least one, each size once, smallest first */
  sizes: TextureSizeData[];
}

export const isTextureSize = (value: unknown): value is TextureSize => (TEXTURE_SIZES as readonly unknown[]).includes(value);

/**
 * The size of a set to show at a wanted size: the largest one that is not larger, else the smallest there is
 * (a set without a 512 map still shows at the lowest quality). `undefined` for an empty set.
 */
export function textureSizeFor(set: TextureSetData, wanted: number): TextureSizeData | undefined {
  const sizes = [...set.sizes].sort((a, b) => a.size - b.size);
  return sizes.filter(entry => entry.size <= wanted).pop() ?? sizes[0];
}

/** What is wrong with an asset's `textures` (config guard, content build, the editor's save): one line per problem */
export function textureSetProblems(raw: unknown, assetType: unknown): string[] {
  const problems: string[] = [];
  if (assetType !== "glb" && assetType !== "gltf") problems.push("only a model file (glb / gltf) has textures");
  if (!Array.isArray(raw)) return [...problems, "expected a list of texture sets"];
  const materials = new Set<string>();
  raw.forEach((set, i) => {
    if (!isObject(set)) return void problems.push(`[${i}]: expected an object`);
    if (set.material !== undefined && (typeof set.material !== "string" || !set.material)) problems.push(`[${i}].material: expected a material name`);
    const key = typeof set.material === "string" ? set.material : "";
    if (materials.has(key)) problems.push(`[${i}]: a second set for ${key ? `material "${key}"` : "every material"}`);
    materials.add(key);
    if (!Array.isArray(set.sizes) || !set.sizes.length) return void problems.push(`[${i}].sizes: expected at least one size`);
    const seen = new Set<unknown>();
    set.sizes.forEach((entry, j) => {
      if (!isObject(entry)) return void problems.push(`[${i}].sizes[${j}]: expected an object`);
      if (!isTextureSize(entry.size)) problems.push(`[${i}].sizes[${j}].size: ${JSON.stringify(entry.size)} is not one of ${TEXTURE_SIZES.join(", ")}`);
      else if (seen.has(entry.size)) problems.push(`[${i}].sizes[${j}].size: ${entry.size} is there twice`);
      seen.add(entry.size);
      if (typeof entry.src !== "string" || !entry.src) problems.push(`[${i}].sizes[${j}].src: expected a file`);
    });
  });
  return problems;
}
