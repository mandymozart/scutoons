import { type Material, type Mesh, type Object3D, SRGBColorSpace, type Texture, TextureLoader } from "three";
import { textureSizeFor, type TextureSetData, type TextureSize } from "@shared/types/textures";
import type { TextureQuality } from "../types/graphics";

/**
 * A model's maps by the reader's texture quality (#260, content schema 2.27). The model file carries its smallest
 * map and shows with it at once; this loads the size the quality asks for and puts it in place of the file's map –
 * on the materials of the loaded model, which every copy of it shares. A change of the quality swaps again.
 *
 * Rules: a set without `material` is the map of every material that has one; the size shown is the largest one not
 * above the wanted size, else the set's smallest; a size the material already shows is not loaded again; the map
 * that leaves is disposed (GPU memory); a map that arrives after a newer request, or after the model was released,
 * is dropped.
 */

/** What the device tells about itself, for `auto` (all optional: browsers differ) */
export interface DeviceHints {
  /** GB of RAM, rounded by the browser (`navigator.deviceMemory`, Chromium) */
  memory?: number;
  /** The reader asked the browser to save data */
  saveData?: boolean;
  /** `navigator.connection.effectiveType` */
  connection?: string;
  /** The largest texture the graphics card takes (px) */
  maxTextureSize?: number;
}

export const deviceHints = (maxTextureSize?: number): DeviceHints => {
  const nav = (typeof navigator === "undefined" ? {} : navigator) as Navigator & {
    deviceMemory?: number;
    connection?: { saveData?: boolean; effectiveType?: string };
  };
  return { memory: nav.deviceMemory, saveData: nav.connection?.saveData, connection: nav.connection?.effectiveType, maxTextureSize };
};

/**
 * The map size (px) a quality asks for on this device. `auto`: 512 on a data-saving or slow (2g / 3g) connection,
 * 1024 on a device with up to 4 GB, else 2048 – 4096 only when the reader chooses it (an uncompressed 4K map takes
 * about 85 MB of graphics memory). Never above what the graphics card takes.
 */
export function wantedTextureSize(quality: TextureQuality | undefined, device: DeviceHints = {}): TextureSize {
  const auto = (): TextureSize => {
    if (device.saveData || /(^|-)(2g|3g)$/.test(device.connection ?? "")) return 512;
    if (device.memory !== undefined && device.memory <= 4) return 1024;
    return 2048;
  };
  const wanted = quality === undefined || quality === "auto" ? auto() : quality;
  const limit = device.maxTextureSize ?? Infinity;
  return ([4096, 2048, 1024, 512] as const).find(size => size <= wanted && size <= limit) ?? 512;
}

type Mapped = Material & { map?: Texture | null };

const materialsOf = (root: Object3D): Mapped[] => {
  const found = new Set<Mapped>();
  root.traverse(object => {
    const material = (object as Mesh).material;
    (Array.isArray(material) ? material : material ? [material] : []).forEach(m => found.add(m as Mapped));
  });
  return [...found];
};

const widthOf = (texture: Texture | null | undefined): number | undefined =>
  (texture?.image as { width?: number } | undefined)?.width;

/** Loads one map file; replaced in tests */
export type TextureFileLoader = (src: string) => Promise<Texture>;
const loadFile: TextureFileLoader = src => new TextureLoader().setCrossOrigin("anonymous").loadAsync(src);

/** The run of `applyTextureSets` that counts for a model: a newer call, or the model's release, ends the older one */
const runs = new WeakMap<Object3D, number>();

/** Stop putting maps on this model (it is being disposed) */
export const cancelTextureSets = (root: Object3D): void => void runs.set(root, (runs.get(root) ?? 0) + 1);

/**
 * Put the maps of `sets` at the wanted size on the model's materials. Resolves when every map is in place (or
 * failed: the material keeps the map it has, with a warning).
 */
export async function applyTextureSets(root: Object3D, sets: readonly TextureSetData[], wanted: number, load: TextureFileLoader = loadFile): Promise<void> {
  const run = (runs.get(root) ?? 0) + 1;
  runs.set(root, run);
  const materials = materialsOf(root);
  await Promise.all(sets.map(async set => {
    const size = textureSizeFor(set, wanted);
    const targets = materials.filter(material => material.map && (set.material === undefined || material.name === set.material));
    if (!size || !targets.length || targets.every(material => widthOf(material.map) === size.size)) return;
    let texture: Texture;
    try {
      texture = await load(size.src);
    } catch (error) {
      console.warn(`[model-textures] Could not load ${size.src}:`, error);
      return;
    }
    if (runs.get(root) !== run) return texture.dispose();
    const replaced = new Set<Texture>();
    targets.forEach(material => {
      const old = material.map as Texture;
      // As the loader set the file's own map: sRGB colour, the glTF orientation, its wrapping and UV channel
      texture.colorSpace = SRGBColorSpace;
      texture.flipY = old.flipY;
      texture.wrapS = old.wrapS;
      texture.wrapT = old.wrapT;
      texture.channel = old.channel;
      texture.anisotropy = old.anisotropy;
      texture.offset.copy(old.offset);
      texture.repeat.copy(old.repeat);
      texture.rotation = old.rotation;
      texture.center.copy(old.center);
      material.map = texture;
      material.needsUpdate = true;
      replaced.add(old);
    });
    texture.needsUpdate = true;
    // A map other materials of the model still show stays
    const inUse = new Set(materials.map(material => material.map));
    replaced.forEach(old => !inUse.has(old) && old.dispose());
  }));
}
