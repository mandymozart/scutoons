import * as THREE from "three";
import { SRGBColorSpace, Texture, TextureLoader, VideoTexture } from "three";
import { type ComponentParamSpec, resolveSpecParams, textureSource, type TextureSpec } from "../types/index";
import { contentUrl } from "../content/game-config";
import { playMedia } from "./media";

/**
 * Texture parameters: a spec's parameter of kind `texture` takes a file – a Library
 * image or video, a file of the content – or a procedural texture a plugin registered (`api.registerTexture`, drawn by
 * its code: a canvas or a data texture, animated or not). Wherever a plugin's code gets its parameters (a material's
 * `build`, a node type's `build`, a component's `apply`, a layer's GLSL) the engine resolves such a value into a
 * three.js `Texture` first (`resolveTextures`): the code never loads a file itself.
 *
 * A **file** texture is shared by every use of the same file (counted, disposed with its last use) – change its wrap or
 * repeat once, never dispose it. A video plays muted in a loop while it is used (unless the editor holds the scene's
 * media, `media.ts`). A **procedural** texture is drawn per
 * use: the use owns it (its `update` runs every frame, it is disposed with the node).
 */

/** What a procedural texture's `draw` gets: its parameters resolved (the spec's defaults filled in), three.js */
export interface TextureContext<P = Record<string, unknown>> {
  params: P;
  /** The app's three.js (one instance – plugins never import three themselves) */
  three: typeof THREE;
}

/** What `draw` returns: the texture and, for an animated one, an `update` every frame; `dispose` frees what it made beside it */
export interface TextureInstance {
  texture: Texture;
  update?(delta: number): void;
  dispose?(): void;
}

/** Draws a procedural texture: a texture (a `CanvasTexture`, a `DataTexture`) or a `TextureInstance` */
export type TextureDraw<P = Record<string, unknown>> = (context: TextureContext<P>) => TextureInstance | Texture | null | void;

export interface TextureRegistration { plugin: string; spec: TextureSpec; draw: TextureDraw }

const registrations = new Map<string, TextureRegistration>();

/** The plugin API registers and unregisters procedural textures here (`plugins.ts`) */
export const setTextureRegistration = (type: string, registration: TextureRegistration): void => {
  registrations.set(type, registration);
};
export const deleteTextureRegistration = (type: string): void => {
  registrations.delete(type);
};
export const textureRegistration = (type: string): TextureRegistration | undefined => registrations.get(type);

/** A texture in use: null when the value names none or it could not be made; `dispose` gives it back */
export interface TextureUse {
  texture: Texture | null;
  update?(delta: number): void;
  dispose(): void;
}

const VIDEO_FILE = /\.(mp4|webm|mov|m4v)([?#].*)?$/i;

interface FileTexture { texture: Texture; element?: HTMLVideoElement; users: number }
const files = new Map<string, FileTexture>();

/** Hidden home for the video elements of video textures (iOS decodes frames more reliably for elements in the DOM) */
const videoHome = (): HTMLElement => {
  let el = document.getElementById("ar-texture-media");
  if (!el) {
    el = document.createElement("div");
    el.id = "ar-texture-media";
    el.setAttribute("aria-hidden", "true");
    Object.assign(el.style, { position: "absolute", width: "0", height: "0", overflow: "hidden" });
    document.body.appendChild(el);
  }
  return el;
};

const loadFile = (src: string): FileTexture => {
  const url = contentUrl(src);
  if (VIDEO_FILE.test(src)) {
    const element = document.createElement("video");
    element.crossOrigin = "anonymous";
    element.muted = true;
    element.loop = true;
    element.playsInline = true;
    element.setAttribute("playsinline", "");
    element.src = url;
    videoHome().appendChild(element);
    void playMedia(element).catch(() => undefined);
    const texture = new VideoTexture(element);
    texture.colorSpace = SRGBColorSpace;
    return { texture, element, users: 0 };
  }
  const texture = new TextureLoader().setCrossOrigin("anonymous").load(url, undefined, undefined, error => console.warn(`[textures] Could not load ${src}:`, error));
  texture.colorSpace = SRGBColorSpace;
  return { texture, users: 0 };
};

const useFile = (src: string): TextureUse => {
  let file = files.get(src);
  if (!file) {
    file = loadFile(src);
    files.set(src, file);
  }
  file.users += 1;
  const used = file;
  let released = false;
  return {
    texture: used.texture,
    dispose: () => {
      if (released) return;
      released = true;
      used.users -= 1;
      if (used.users > 0) return;
      files.delete(src);
      used.texture.dispose();
      if (used.element) {
        used.element.pause();
        used.element.removeAttribute("src");
        used.element.load?.();
        used.element.remove();
      }
    },
  };
};

/** The videos file textures show now – the scene holds them with its other media */
export const textureMedia = (): HTMLVideoElement[] =>
  [...files.values()].flatMap(file => (file.element ? [file.element] : []));

const NONE: TextureUse = { texture: null, dispose: () => undefined };

/** The texture a value names (`TextureValue`); `where` names the use in the log */
export const useTexture = (value: unknown, where: string): TextureUse => {
  const source = textureSource(value);
  if (source.kind === "none") return NONE;
  if (source.kind === "file") return useFile(source.src);
  const registration = registrations.get(source.type);
  if (!registration) {
    console.warn(`[textures] No procedural texture "${source.type}" registered (${where}) – none`);
    return NONE;
  }
  let drawn: TextureInstance | Texture | null | void;
  try {
    drawn = registration.draw({ params: resolveSpecParams(registration.spec.params, source.params), three: THREE });
  } catch (error) {
    console.error(`[textures] ${source.type} (plugin ${registration.plugin}) failed (${where}):`, error);
    return NONE;
  }
  const instance: TextureInstance | null = drawn instanceof Texture ? { texture: drawn } : drawn && drawn.texture instanceof Texture ? drawn : null;
  if (!instance) {
    console.error(`[textures] ${source.type} (plugin ${registration.plugin}) returned no three.js texture (${where})`);
    return NONE;
  }
  return {
    texture: instance.texture,
    update: instance.update ? delta => instance.update!(delta) : undefined,
    dispose: () => {
      instance.dispose?.();
      instance.texture.dispose();
    },
  };
};

/** A spec's parameters with every texture value resolved to its texture; `update` and `dispose` cover the textures */
export interface ResolvedTextures {
  values: Record<string, unknown>;
  update(delta: number): void;
  dispose(): void;
}

/** Resolve the `texture` parameters of `values` (resolved against `specParams` already) into textures */
export const resolveTextures = (specParams: Record<string, ComponentParamSpec>, values: Record<string, unknown>, where: string): ResolvedTextures => {
  const uses: TextureUse[] = [];
  const resolved = { ...values };
  for (const [name, param] of Object.entries(specParams)) {
    if (param.kind !== "texture") continue;
    const use = useTexture(values[name], `${where} ${name}`);
    uses.push(use);
    resolved[name] = use.texture;
  }
  return {
    values: resolved,
    update: delta => uses.forEach(use => use.update?.(delta)),
    dispose: () => uses.forEach(use => use.dispose()),
  };
};

/** The names of a spec's texture parameters */
export const textureParams = (specParams: Record<string, ComponentParamSpec>): string[] =>
  Object.entries(specParams).filter(([, param]) => param.kind === "texture").map(([name]) => name);

/** The texture parameters of two resolved value sets name the same textures (else the user rebuilds) */
export const sameTextures = (specParams: Record<string, ComponentParamSpec>, a: Record<string, unknown>, b: Record<string, unknown>): boolean =>
  textureParams(specParams).every(name => JSON.stringify(a[name] ?? "") === JSON.stringify(b[name] ?? ""));
