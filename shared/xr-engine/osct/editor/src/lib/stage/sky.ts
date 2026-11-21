/**
 * The stage's sky and light (#185, three-free): an environment replaces the HDRI sky box.
 *
 * - **The world.** With an experience's environment (its draft's, or edits not yet saved) the stage shows the reader's
 *   look-around world as its sky while the shading is rendered; while the Environment tab is open it shows the world in
 *   every shading (the tab's preview). Without one: the HDRI while rendered.
 * - **The light** follows the reader: the reader lights the AR content with the engine's own lights (ambient and sun,
 *   `engine/src/scene/view.ts`) and the content's light nodes – its look-around world is a full-screen pass under the
 *   scene and lights nothing. So while the world is the sky, the HDRI does not light: the scene's own lights
 *   at full strength, as in the reader. Without a world the HDRI lights the stage while rendered (the studio's light).
 * - **The camera window.** The world leaves a window around the scene (and the open roof) where the reader shows its
 *   camera picture; on the stage the HDRI fills it while rendered, drawn behind the world – a stand-in for the camera
 *   picture, as the desk is, not a light. The reader is unchanged (camera picture there, no HDRI).
 */

import type { EnvironmentData } from "@shared/types/environment";
import type { HdriId, ShadingMode } from "./preview";
import { fileNameOf } from "#lib/library-place.js";

/** What the stage shows as sky: the environment's world, the HDRI on its sphere, or nothing (the editor's dark ground) */
export type StageSky = "world" | "hdri" | "none";

/** What lights the stage: the HDRI (with the scene's lights stepped back) or the scene's own lights, as in the reader */
export type StageLight = "hdri" | "scene";

export interface StageLook {
  sky: StageSky;
  light: StageLight;
  /** The HDRI to load – null when it neither shows (as sky or behind the world) nor lights */
  hdri: HdriId | null;
  /**
   * The HDRI fills the world's window behind it – the stage's stand-in for the reader's camera picture there (the
   * world's pass leaves it transparent) – as a background only, no light
   */
  behindWorld: boolean;
}

/**
 * The experience's environment as the stage takes it: the edited values when the experience has one (`saved`) or the author has
 * started one (edited away from the default) – else null, the experience has none and the HDRI stays.
 */
export const experienceEnvironment = (saved: EnvironmentData | undefined, edited: EnvironmentData, dirty: boolean): EnvironmentData | null =>
  saved || dirty ? edited : null;

/**
 * The environment's world is drawn under the stage: always while the Environment tab previews it, else while the experience
 * has one and the shading is rendered (it takes the HDRI's place, which shows only while rendered)
 */
export const showsWorld = (input: { environment: boolean; previewing: boolean; shading: ShadingMode }): boolean =>
  input.environment && (input.previewing || input.shading === "rendered");

/**
 * Which sky and which light, from the shading, the chosen HDRI and whether the world is drawn. While rendered, the
 * world's window – where the reader shows the camera picture (around the scene, the open roof) – shows the HDRI as a
 * stand-in for the camera picture (`behindWorld`): a background behind the world, never its light.
 */
export const stageLook = (input: { shading: ShadingMode; hdri: HdriId | null; world: boolean }): StageLook => {
  const rendered = input.shading === "rendered" && !!input.hdri;
  if (input.world) return rendered ? { sky: "world", light: "scene", hdri: input.hdri, behindWorld: true } : { sky: "world", light: "scene", hdri: null, behindWorld: false };
  if (rendered) return { sky: "hdri", light: "hdri", hdri: input.hdri, behindWorld: false };
  return { sky: "none", light: "scene", hdri: null, behindWorld: false };
};

/**
 * The environment's sky in a few words, for the HDRI picker it overrides: "the environment's (sky none)",
 * "the environment's (sky dusk.jpg)" – the world is no longer drawn (#310). `nameOf` names a file (the Library's name),
 * else its URL's last part.
 */
export const environmentSkyLabel = (environment: EnvironmentData, nameOf: (url: string) => string | undefined = () => undefined): string => {
  const name = (url: string) => nameOf(url) ?? fileNameOf(url);
  return `the environment's (sky ${environment.skySrc ? name(environment.skySrc) : "none"})`;
};
