/**
 * Scenes (#307, `agents/SCENES.md`, schema 3.0): the labels the reader shows for a place in the experience. Pure, for the
 * tests – `./game-config.ts` applies them to the loaded configuration.
 *
 * A label is what is printed (free text, never arithmetic): the scene's `label`, or – more specific – the label of the
 * entry's surface. Nothing here derives anything from a number.
 */
import type { EntryData, SceneData, SurfaceData } from "@shared/types/game-config";

/** The surface of a scene with the given id, if the scene has it */
export const surfaceOf = (scene: Pick<SceneData, "surfaces"> | undefined, surfaceId: string | undefined): SurfaceData | undefined =>
  surfaceId === undefined ? undefined : scene?.surfaces.find(s => s.id === surfaceId);

const labelOf = (value: string | undefined): string | null => {
  const label = value?.trim();
  return label ? label : null;
};

/** The label a scene carries – what the switcher and the lists show for it */
export const sceneLabel = (scene: Pick<SceneData, "label" | "id">): string => labelOf(scene.label) ?? scene.id;

/**
 * The place an entry is on, as the reader finds it: the label of its surface when it names one that carries a label,
 * else its scene's label
 */
export const entryPlaceLabel = (
  entry: Pick<EntryData, "surface">,
  scene: Pick<SceneData, "id" | "label" | "surfaces"> | undefined,
): string => labelOf(surfaceOf(scene, entry.surface)?.label) ?? (scene ? sceneLabel(scene) : "");
