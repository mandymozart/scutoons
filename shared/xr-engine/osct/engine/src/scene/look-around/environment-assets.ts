import { type CubeTexture, LinearFilter, SRGBColorSpace, type Texture, TextureLoader, type WebGLRenderer } from "three";
import type { EnvironmentData } from "@shared/types/environment";
import { contentUrl } from "../../content/game-config";

/** How the files are fetched – the tests pass their own */
export interface EnvironmentLoaders {
  /** An equirectangular image as a texture (sRGB) */
  sky(url: string): Promise<Texture>;
}

export const DEFAULT_ENVIRONMENT_LOADERS: EnvironmentLoaders = {
  sky: async url => {
    const texture = await new TextureLoader().setCrossOrigin("anonymous").loadAsync(url);
    texture.colorSpace = SRGBColorSpace;
    // Sampled by direction in the pass: no mipmaps, so the image's seam (behind the reader) shows no line
    texture.generateMipmaps = false;
    texture.minFilter = LinearFilter;
    return texture;
  },
};


/**
 * The environment's assets for the look-around pass: the sky image. **The world is gone** (Tilman, 2026-11-18 – first the
 * built-in café, then the world model too, #310): a content's `worldSrc` is ignored, nothing is drawn around the surfaces but
 * the sky. `set()` loads a changed sky and frees the one no longer named; a newer `set()` wins over a load still running.
 * While the sky loads – or when it fails (logged) – nothing is drawn.
 */
export class EnvironmentAssets {
  /** The sky image, null = none */
  sky: Texture | null = null;
  /** Always null: the world model is no longer drawn (kept for the pass's uniforms) */
  readonly world: CubeTexture | null = null;
  /** Called when `sky` changed */
  onChange: () => void = () => {};

  private skySrc: string | null = null;
  private disposed = false;

  constructor(_renderer: WebGLRenderer, private loaders: EnvironmentLoaders = DEFAULT_ENVIRONMENT_LOADERS) {}

  /** Load the environment's sky (a reference as the content holds it, resolved with `contentUrl`) */
  set(environment: EnvironmentData): Promise<void> {
    const skySrc = environment.skySrc ? contentUrl(environment.skySrc) : null;
    if (skySrc === this.skySrc) return Promise.resolve();
    this.skySrc = skySrc;
    this.setSky(null);
    return skySrc ? this.loadSky(skySrc) : Promise.resolve();
  }

  dispose(): void {
    this.disposed = true;
    this.setSky(null);
  }

  private async loadSky(url: string): Promise<void> {
    try {
      const texture = await this.loaders.sky(url);
      if (this.disposed || this.skySrc !== url) {
        texture.dispose();
        return;
      }
      this.setSky(texture);
    } catch (error) {
      console.warn(`[look-around] The sky "${url}" could not be loaded – none is drawn:`, error);
    }
  }

  private setSky(texture: Texture | null): void {
    if (this.sky === texture) return;
    this.sky?.dispose();
    this.sky = texture;
    this.onChange();
  }
}
