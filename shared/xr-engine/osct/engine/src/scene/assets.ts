import { type AnimationClip, type Group, type Material, type Mesh, type Object3D, SRGBColorSpace, Texture, TextureLoader, VideoTexture } from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { MeshoptDecoder } from "three/examples/jsm/libs/meshopt_decoder.module.js";
import type { AssetData } from "../types/index";
import { loadCompressed } from "../content/compressed";
import { applyTextureSets, cancelTextureSets } from "./model-textures";

/**
 * The AR scene's assets by id. Loaded once per id, kept while a scene uses them (a scene switch keeps
 * the assets both scenes use), disposed (GPU memory) when released. Requests hit the browser cache the
 * PreloaderService filled.
 */

export type LoadedAsset =
  | { assetType: "glb" | "gltf"; scene: Group; animations: AnimationClip[] }
  | { assetType: "image"; texture: Texture }
  | { assetType: "video"; element: HTMLVideoElement; texture: VideoTexture }
  | { assetType: "audio"; element: HTMLAudioElement };

interface Entry {
  data: AssetData;
  loading: Promise<LoadedAsset | null>;
  loaded: LoadedAsset | null;
  /** The file loaded – the level the media quality chose (#331), else the asset's own `src` */
  src: string;
}

/**
 * A playing video or sound changes its level (#331): the same element takes the other file and goes on at the same time,
 * playing if it played – the VideoTexture and the entity keep the element, so nothing else is rebuilt.
 */
export const swapMediaSrc = (element: HTMLMediaElement, src: string): void => {
  const at = element.currentTime;
  const playing = !element.paused;
  element.src = src;
  const resume = () => {
    if (at && (!element.duration || at < element.duration)) element.currentTime = at;
    if (playing) void element.play().catch(() => {});
  };
  element.addEventListener("loadedmetadata", resume, { once: true });
  element.load();
};

/** A picture changes its level (#331): the same texture takes the other file once it loaded, so the materials keep it */
const swapImageSrc = async (texture: Texture, src: string): Promise<void> => {
  const next = await new TextureLoader().setCrossOrigin("anonymous").loadAsync(src);
  texture.image = next.image;
  texture.needsUpdate = true;
  next.dispose();
};

let gltfLoader: GLTFLoader | null = null;
const getGltfLoader = (): GLTFLoader => {
  if (!gltfLoader) {
    gltfLoader = new GLTFLoader();
    gltfLoader.setMeshoptDecoder(MeshoptDecoder); // models compressed with gltf-transform (meshopt)
  }
  return gltfLoader;
};

/** Hidden home for media elements (iOS decodes video frames more reliably for elements in the DOM) */
const mediaContainer = (): HTMLElement => {
  let el = document.getElementById("ar-media");
  if (!el) {
    el = document.createElement("div");
    el.id = "ar-media";
    el.setAttribute("aria-hidden", "true");
    Object.assign(el.style, { position: "absolute", width: "0", height: "0", overflow: "hidden" });
    document.body.appendChild(el);
  }
  return el;
};

/**
 * Where each video stopped when its asset was released (a scene switch, the scene disposed), by file: loaded again –
 * the reader came back to its page – it goes on from there, not from the start (#290). Kept for the page's life.
 */
const positions = new Map<string, number>();

/** The time a video file stopped at when it was last released (0: never, or at its start) */
export const videoPosition = (src: string): number => positions.get(src) ?? 0;

/** Start a new video element where its file stopped last (once its duration is known) */
const resumeAt = (element: HTMLVideoElement, src: string): void => {
  const at = positions.get(src);
  if (!at) return;
  const seek = () => {
    if (element.duration && at < element.duration) element.currentTime = at;
  };
  if (element.readyState >= 1) seek();
  else element.addEventListener("loadedmetadata", seek, { once: true });
};

const media = <T extends HTMLMediaElement>(tag: "video" | "audio", data: AssetData, src: string): T => {
  const el = document.createElement(tag) as T;
  el.id = data.id;
  el.crossOrigin = "anonymous";
  el.preload = "auto";
  if (el instanceof HTMLVideoElement) {
    el.loop = true;
    el.playsInline = true;
    el.setAttribute("playsinline", "");
    el.setAttribute("webkit-playsinline", "");
  }
  el.src = src;
  mediaContainer().appendChild(el);
  return el;
};

/**
 * A model file by its URL – .glb: the content build's gzip copy, unpacked (hosts serve .glb uncompressed); .gltf: its
 * own files
 */
export const loadGltf = async (src: string, assetType: "glb" | "gltf"): Promise<{ scene: Group; animations: AnimationClip[] }> => {
  const gltf = assetType === "glb"
    ? await getGltfLoader().parseAsync(await loadCompressed(src), src.replace(/[^/]*$/, ""))
    : await getGltfLoader().loadAsync(src);
  return { scene: gltf.scene, animations: gltf.animations };
};

/**
 * One asset. `src`: the file to load – the level the reader's media quality chose (#331), else the asset's own; a video
 * resumes by its asset's `src`, so a change of level goes on where it stopped.
 */
const load = async (data: AssetData, src: string = data.src): Promise<LoadedAsset | null> => {
  switch (data.assetType) {
    case "glb":
    case "gltf":
      return { assetType: data.assetType, ...(await loadGltf(data.src, data.assetType)) };
    case "image": {
      const texture = await new TextureLoader().setCrossOrigin("anonymous").loadAsync(src);
      texture.colorSpace = SRGBColorSpace;
      return { assetType: "image", texture };
    }
    case "video": {
      const element = media<HTMLVideoElement>("video", data, src);
      resumeAt(element, data.src);
      const texture = new VideoTexture(element);
      texture.colorSpace = SRGBColorSpace;
      return { assetType: "video", element, texture };
    }
    case "audio":
      return { assetType: "audio", element: media<HTMLAudioElement>("audio", data, src) };
    default:
      console.warn(`[assets] Unknown asset type ${(data as AssetData).assetType} (${(data as AssetData).id})`);
      return null;
  }
};

/** Free the GPU memory of a loaded model (geometries, materials, their textures) */
export const disposeObject = (root: Object3D): void => {
  root.traverse(object => {
    const mesh = object as Mesh;
    mesh.geometry?.dispose();
    const materials: Material[] = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
    materials.forEach(material => {
      Object.values(material).forEach(value => {
        if (value instanceof Texture) value.dispose();
      });
      material.dispose();
    });
  });
};

const dispose = (asset: LoadedAsset, src: string): void => {
  switch (asset.assetType) {
    case "glb":
    case "gltf":
      cancelTextureSets(asset.scene); // a map still loading for it is dropped
      disposeObject(asset.scene);
      break;
    case "image":
      asset.texture.dispose();
      break;
    case "video":
    case "audio":
      if (asset.assetType === "video") {
        asset.texture.dispose();
        if (asset.element.currentTime > 0) positions.set(src, asset.element.currentTime);
      }
      asset.element.pause();
      asset.element.removeAttribute("src");
      asset.element.load(); // release the media buffer
      asset.element.remove();
      break;
  }
};

export class AssetStore {
  private entries = new Map<string, Entry>();

  /**
   * `textureSize`: the map size (px) models with a texture set are shown at – asked on every load and refresh.
   * `mediaSrc`: the file of an image, video or sound with levels to load (#331, `chooseMediaSrc`) – asked when it loads;
   * none = the asset's own `src`.
   */
  constructor(private textureSize: () => number = () => 512, private mediaSrc: (data: AssetData) => string = data => data.src) {}

  /** A model with a texture set shows with its file's map at once; the wanted size follows (#260) */
  private applyTextures(entry: Entry): void {
    const { loaded, data } = entry;
    if (!loaded || !data.textures?.length || (loaded.assetType !== "glb" && loaded.assetType !== "gltf")) return;
    void applyTextureSets(loaded.scene, data.textures, this.textureSize());
  }

  /** The wanted map size changed (the reader's texture quality): swap the maps of every loaded model */
  refreshTextures(): void {
    this.entries.forEach(entry => this.applyTextures(entry));
  }

  /**
   * The reader's media quality changed (#331): every loaded picture, video and sound with levels whose chosen file is
   * another takes it – a video or sound goes on at the same time, playing if it played; a picture swaps in place.
   */
  refreshMedia(): void {
    this.entries.forEach(entry => {
      const { data, loaded } = entry;
      if (!loaded || !data.levels?.length) return;
      const src = this.mediaSrc(data);
      if (src === entry.src) return;
      entry.src = src;
      if (loaded.assetType === "video" || loaded.assetType === "audio") swapMediaSrc(loaded.element, src);
      else if (loaded.assetType === "image") void swapImageSrc(loaded.texture, src).catch(error => console.warn(`[assets] Could not load ${data.id} (${src}):`, error));
    });
  }

  /** Load assets (known ids are reused). A failed asset resolves to null – its entity is skipped. */
  async load(assets: readonly AssetData[]): Promise<void> {
    await Promise.all(assets.map(data => {
      let entry = this.entries.get(data.id);
      if (!entry) {
        const src = data.levels?.length ? this.mediaSrc(data) : data.src;
        const created: Entry = { data, loaded: null, loading: Promise.resolve(null), src };
        created.loading = load(data, src)
          .then(loaded => {
            created.loaded = loaded;
            this.applyTextures(created);
            return loaded;
          })
          .catch(error => {
            console.warn(`[assets] Could not load ${data.id} (${data.src}):`, error);
            return null;
          });
        entry = created;
        this.entries.set(data.id, entry);
      }
      return entry.loading;
    }));
  }

  get(id: string): LoadedAsset | undefined {
    return this.entries.get(id)?.loaded ?? undefined;
  }

  /** The store was asked for this asset (loaded, loading or failed) – a file of the scene's content */
  knows(id: string): boolean {
    return this.entries.has(id);
  }

  /** Every loaded video and sound element – what the editor's hold pauses (`media.ts`) */
  media(): HTMLMediaElement[] {
    const media: HTMLMediaElement[] = [];
    this.entries.forEach(({ loaded }) => {
      if (loaded?.assetType === "video" || loaded?.assetType === "audio") media.push(loaded.element);
    });
    return media;
  }

  /** The loaded videos with their files – what the scene's subtitles follow */
  videos(): Array<{ src: string; element: HTMLVideoElement }> {
    const videos: Array<{ src: string; element: HTMLVideoElement }> = [];
    this.entries.forEach(({ data, loaded }) => {
      if (loaded?.assetType === "video") videos.push({ src: data.src, element: loaded.element });
    });
    return videos;
  }

  /** Dispose every asset whose id is not in `keep` */
  release(keep: Set<string> = new Set()): void {
    this.entries.forEach((entry, id) => {
      if (keep.has(id)) return;
      this.entries.delete(id);
      // Still loading: dispose when it arrives
      void entry.loading.then(loaded => loaded && dispose(loaded, entry.data.src));
    });
  }
}
