/**
 * The reader's graphics options the scene's surroundings follow (`scene/look-around`). The app owns the setting
 * (the reader: `GraphicsService`, stored on the device); the engine only reads it and listens for changes.
 */
import type { TextureSize } from "@shared/types/textures";

/** The size of the models' maps (#260): `auto` goes by the device and its connection, a number is the largest size loaded */
export type TextureQuality = "auto" | TextureSize;

/**
 * The quality of pictures, videos and sounds that come in levels (#331, content schema 4.3): `auto` goes by the screen,
 * the device and its connection; `low` / `medium` / `high` take the smallest, the middle and the largest level.
 */
export const MEDIA_QUALITIES = ["auto", "low", "medium", "high"] as const;
export type MediaQuality = (typeof MEDIA_QUALITIES)[number];
export const isMediaQuality = (value: unknown): value is MediaQuality => (MEDIA_QUALITIES as readonly unknown[]).includes(value);

export interface GraphicsSettings {
  /** Looking up, sky-like parts of the camera picture show the world's sky */
  onionSky: boolean;
  /** The world around the reader, anchored to the found surfaces */
  surroundings: boolean;
  /** Optional: how large the maps of models with a texture set are loaded (none = `auto`) */
  textureQuality?: TextureQuality;
  /** Optional: which level of pictures, videos and sounds with levels is loaded (none = `auto`) */
  mediaQuality?: MediaQuality;
}

export interface GraphicsSource {
  getSettings(): GraphicsSettings;
  /** Called on every change; returns the unsubscribe function */
  subscribe(listener: (settings: GraphicsSettings) => void): () => void;
}

/** No surroundings, no sky (an app that passes no graphics source – the editor's stage) */
export const NO_GRAPHICS: GraphicsSource = {
  getSettings: () => ({ onionSky: false, surroundings: false }),
  subscribe: () => () => {},
};
