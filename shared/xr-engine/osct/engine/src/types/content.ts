/**
 * The content the engine works with: the contract of `shared/` (what the content build writes) and the app models
 * `content/game-config.ts` maps it to – a target knows its entry and scene, an entry its scene, a scene its
 * entries and targets, and entity refs are resolved.
 */
export * from "@shared/types/game-config";
export { EntryCategory, ENTRY_CATEGORIES } from "@shared/types/entry";
import type { EntityData, EntryData, SceneData, TargetData } from "@shared/types/game-config";

/** App model of a target (mapped from `TargetData`); the entity ref is resolved */
export interface Target extends Omit<TargetData, "entity"> {
  entryId: string;
  sceneId: string;
  entity?: EntityData;
}

/** App model of an entry (mapped from `EntryData`): `sceneId` is its `scene` */
export interface Entry extends Omit<EntryData, "target"> {
  sceneId: string;
  target?: Target;
}

/** App model of a scene (mapped from `SceneData`; the app's name for it is still `Scene`) */
export interface Scene extends SceneData {
  /** Every entry of the scene, with or without target */
  entries: Entry[];
  /** In entry order */
  targets: Target[];
}
