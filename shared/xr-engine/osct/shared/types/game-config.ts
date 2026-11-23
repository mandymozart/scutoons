/**
 * Game configuration contract – the shape of `game.config.json`.
 *
 * Written by the content build (`scripts/`), read by the engine (`engine/src/content/game-config.ts`), which the reader
 * and the editor load it with.
 * Defined once here (RULES #13). `*Data` = data loaded from JSON; the app maps it to its own model.
 * Runtime checks: `shared/guards/game-config.ts`.
 */

import type { EntryCategory } from "./entry";
import type { FilterData } from "./filters";
import type { ComponentData } from "./components";
import type { ExperienceAppearance } from "./appearance";
import type { EnvironmentData } from "./environment";
import type { LightData } from "./lights";
import type { TextureSetData } from "./textures";
import type { MediaLevelData } from "./media-levels";
import type { PassData } from "./passes";
import type { PluginData } from "./plugins";
import type { MaterialData } from "./materials";
import type { SubtitleData } from "./subtitles";
import type { LogicData } from "./logic";
import type { VariableDeclaration } from "./variables";

export * from "./filters";
export * from "./placement";
export * from "./components";
export * from "./behaviour";
export * from "./address";
export * from "./logic";
export * from "./variables";
export * from "./lights";
export * from "./textures";
export * from "./media-levels";
export * from "./passes";
export * from "./appearance";
export * from "./plugins";
export * from "./node-types";
export * from "./subtitles";
export * from "./materials";
export * from "./spec-builders";

/**
 * Node types of an entity (extensible, RULES #7): what is seen – a model, a video or image plane, a line of text,
 * a light – or a `group` that only holds children. Links are entries, not entities.
 * The guard lets a node type it does not know pass (the engine renders nothing for it but keeps its children), so
 * a later type is a MINOR addition; the content build and the editor's API still refuse one – unless a plugin
 * registers it (schema 2.13, `./node-types.ts`): ENTITY_TYPES stays the built-in list.
 */
export const ENTITY_TYPES = ["group", "model", "video", "image", "text", "light"] as const;
export type EntityType = (typeof ENTITY_TYPES)[number];
/** A node's type: a built-in one, or one a plugin registers (`registerNodeType`, camelCase – `./node-types.ts`) */
export type NodeType = EntityType | (string & {});

export const ASSET_TYPES = ["glb", "gltf", "video", "image", "audio"] as const;
export type AssetType = (typeof ASSET_TYPES)[number];

/**
 * Content schema version `MAJOR.MINOR`: the version of this contract (shared/), the only compatibility rule
 * between content and app (RULES #10). The app version a content build was made with is information only.
 * - MAJOR: a change an older app cannot read – a new required field, a renamed or removed field, a changed
 *   type, a new enum value (asset type, category). The app refuses content of a newer MAJOR at startup and of a MAJOR
 *   before `OLDEST_READABLE_SCHEMA_MAJOR`.
 * - MINOR: an addition an older app of the same MAJOR still loads (RULES #10): what it does not know of a kind it knows
 *   is skipped with a warning (`engine/src/content/forward-compatible.ts`), never refused.
 * The history of the 1.x and 2.x MINORs is the table in docs/content.md ("Schema and app versions").
 *
 * **3.0** (#319, `agents/SCENES.md` "Schema 3.0" – the first release, no reader of 2.x was shipped): one clean model of
 * scenes. The configuration's `scenes` (was `spreads`) and `initialScene` (was `initialSpreadId`); a scene has a `label`,
 * an optional `group` and its `surfaces` (at least one, each placed in the scene – `place`); an entry names its `scene`
 * (and a `surface` when the scene has several); a target's `place` lies on a surface (was `print`). Gone: page numbers
 * everywhere (`page`, `firstPage`, `lastPage`, `print.page`), a spread's `title`, `format`, `matter` and `layout`, a
 * surface's `side`, and the configuration's `tutorial` (the onboarding of 2.24 apps).
 *
 * **4.0** (#320, `agents/REFACTOR-EXPERIENCE.md`): the experience is the top-level object – the configuration's
 * `experience` (was `book`); the printed book is optional data on it, `experience.book` (`BookData`: `author`,
 * `publisher` – these were on the top level). `sourceOfTruth` is gone: every experience adds and removes its scenes. Behaviour scope and plugin variable scope `experience`
 * (was `book`), text nodes' `{{experience.title}}` (was `{{book.title}}`); `latest.json` / `versions.json` name the
 * `experience` (was `book`).
 *
 * **4.1** (#306): a target's optional `recognition` – the image tracker's data made ahead from its printed image
 * (`osct targets prepare`); an older app ignores it and makes that data from the image itself, as before.
 *
 * **4.2** (#306): the switch between both paths – the experience's optional `recognition` (`"original"`, the default:
 * every target's data is made from its printed image on the device, as before; `"prepared"`: a target's `recognition`
 * is used where it has one) and a target's optional `recognitionOff` (`true`: always its printed image, even when the
 * experience says prepared). An app of 4.1 ignores both and uses a target's `recognition` wherever it has one.
 *
 * **4.3** (#331): media in quality levels (`./media-levels.ts`) – an image, video or audio asset's optional `levels` and an
 * entry's optional `imageLevels`; the reader loads the level its media quality chooses. An app of 4.2 ignores both and
 * loads the medium's own `src`, as before.
 *
 * **5.0** (#334, agents/research/2026-11-20-logic-nodes-shared-concept.md §5.4): logic beyond the nodes. A rule's `when`
 * is an expression of the experience's expression language (`hasKey && tries > 2`, `../text/expressions.ts`) – the
 * object form (`{ hasKey: true }`, `{ any: … }`, `{ not: … }`) is gone; a scene's and the experience's optional `logic`
 * (`{ rules, canvas? }`, `./logic.ts`) hold rules in the behaviour's grammar that listen to the bus and name nodes by address
 * (`target:<targetId>/<node name>`, `./address.ts`); every rule gains the action `goto` (`scene:<id>`). The canvas is
 * authoring data and never in a reader's configuration. A `set` value is an expression (`tries + 1`; a text is the
 * language's literal `"calm"`), and variables are typed – declared in the experience's optional `variables` or inferred
 * from their first set (`./variables.ts`). 4.x content is not read (its conditions have the old shape):
 * the content build converts a content folder (`npm run convert:5`), the server migrates its rows (`Migration5.php`).
 *
 * **5.1** (#334 L-T6, §5.9 – plugin API 3.1): ports from the specs. A spec a plugin registers (component, node type, pass,
 * material, texture, look) may declare `actions` (the rule verbs it answers), `events` (the scene events it emits) and
 * `uses` (what its code listens to, emits, reads and sets – `SpecLogic`, `./components.ts`); the build records them in the
 * plugin's `registered`. A number parameter may carry `slider` (its soft range) and `integer`. A logic canvas may carry
 * `exposed` (the ports an author exposed per box, `./logic.ts`). The built-in node types have specs for the Logic view
 * (`BUILT_IN_NODE_SPECS`, `./node-types.ts`). An app of 5.0 ignores the new spec fields (it reads a recorded spec's
 * kinds and params only) and never sees a canvas (a reader's configuration carries none).
 *
 * **5.2** (#334 L-T7): the Logic canvas as the SCUTOONS deck draws it – a logic canvas may carry `positions` (box id →
 * `{ x, y }`: boxes pinned over the automatic layout) and a canvas group `colour` (`#rrggbb`) and `folded` (`./logic.ts`).
 * Authoring data only: a reader's configuration carries no canvas, so an app of 5.1 is not touched.
 *
 * **5.3** (#334): notes on the Logic canvas as the deck's – a canvas note may carry `box` (it stands beside that box, its
 * `x`, `y` an offset), `author` and `view` (the target's view of a scene's canvas it is drawn in, `./logic.ts`). Authoring
 * data only, like 5.2: an app of 5.2 is not touched.
 *
 * **6.0** (#341, plugin API 4.0 – Tilman, 2026-11-21: "you use the word hear for events. i think it is listen"): a
 * recorded spec's bus use is `uses.listens` (was `uses.hears`, `./components.ts`). A rename, so a MAJOR: an app of 6.0
 * reads 6.x only (`OLDEST_READABLE_SCHEMA_MAJOR` 6) and a 5.x app refuses 6.0 – a content source's plugin sources are
 * rewritten (`osct plugins listen --dir`), the server's rows by `Migration6.php`, the stored plugin sources by
 * `osct plugins listen`; every published experience is published again.
 * Every change in shared/ needs a bump: shared/content-schema.json locks the checksum of shared/ to this
 * version (scripts: `npm run schema:lock`, checked by a test).
 */
export const CONTENT_SCHEMA_VERSION = "6.0";
/** The oldest content schema MAJOR this contract reads – 6: no 5.x content is read (6.0 renamed a spec's `uses.hears` to `uses.listens`) */
export const OLDEST_READABLE_SCHEMA_MAJOR = 6;

export interface ContentSchema {
  major: number;
  minor: number;
}

/** `"1.0"` → { major: 1, minor: 0 }; null when it is not `MAJOR.MINOR` */
export const parseContentSchema = (value: unknown): ContentSchema | null => {
  const match = typeof value === "string" ? /^(\d+)\.(\d+)$/.exec(value) : null;
  return match ? { major: Number(match[1]), minor: Number(match[2]) } : null;
};

export interface ConfigurationVersion {
  /** App version the content build was made with – information only, never a reason to refuse */
  version: string;
  timestamp: string; // when the content build inputs last changed
  hash?: string; // checksum of the content build inputs (the content's own version)
  /** Content schema `MAJOR.MINOR` the content was built with (`CONTENT_SCHEMA_VERSION`) */
  schema: string;
}

/**
 * The experience: the top-level object of the content – a book's companion, a wall, a room, an exhibition. Schema 4.0
 * (#320): was `book`; the print data moved into the optional `book`.
 */
export interface ExperienceData {
  id: string; // e.g. "osct" – identifies the experience (storage keys, progress, the Library folder, the editor's URL)
  title: string;
  /** Optional (schema 2.5): the post-processing passes of every scene without a list of its own (`./passes.ts`) */
  passes?: PassData[];
  /** Optional (schema 2.8; world and sky assets 2.9): the world around the reader and the onion sky (`./environment.ts`); none = the default */
  environment?: EnvironmentData;
  /** Optional (schema 2.29): the look an entity appears with on reveal, found and lost (`./appearance.ts`); none = the default look */
  appearance?: ExperienceAppearance;
  /** Optional (schema 4.0): the printed book this experience accompanies – none: not a book (a wall, a room, an exhibition) */
  book?: BookData;
  /**
   * Optional (schema 4.2, #306): how the reader gets its targets' recognition data – `"original"` (none = the default):
   * made from each printed image on the device, the path tested with every target; `"prepared"`: a target's
   * `recognition` made ahead where it has one (and no `recognitionOff`), else its printed image. A device's debug switch
   * may override it (`engine/src/utils/recognition-setting.ts`). Any other text reads as `"original"`.
   */
  recognition?: RecognitionMode;
  /**
   * Optional (schema 5.0, #334): the experience's rules – live for the whole session, across scene switches
   * (`./logic.ts`); its canvas never reaches a reader
   */
  logic?: LogicData;
  /**
   * Optional (schema 5.0, #334 B7): the rules' variables declared – name → `{ type, scope?, default? }` (`./variables.ts`);
   * an undeclared variable takes the type of its first set
   */
  variables?: Record<string, VariableDeclaration>;
}

/** How the reader gets its targets' recognition data (schema 4.2, #306): `ExperienceData.recognition` */
export const RECOGNITION_MODES = ["original", "prepared"] as const;
export type RecognitionMode = (typeof RECOGNITION_MODES)[number];

/** The printed book an experience accompanies (schema 4.0, #320): print data only */
export interface BookData {
  author: string;
  /** Shown on the home page above "Start" (optional) */
  publisher?: string;
}

/**
 * Where a surface lies in its scene (schema 3.0) – the **scene's coordinate system**, one for the whole scene, not per
 * surface: millimetres, the origin on the table at the middle of the scene's first row of surfaces (a double page: its
 * spine), +x to the right, +y up, +z towards the reader (three.js' axes). `x`, `y`, `z` are the surface's centre;
 * unrotated (`rx`, `ry`, `rz` 0, degrees, Euler XYZ) a surface lies flat, face up, its top edge away from the reader
 * (−z) – a page on the table. A wall ahead stands up: `rx` 90, its face towards the reader.
 */
export interface SurfacePlace {
  x: number;
  y: number;
  z: number;
  rx: number;
  ry: number;
  rz: number;
}

/** The six numbers of a `SurfacePlace`, all required */
export const SURFACE_PLACE_KEYS = ["x", "y", "z", "rx", "ry", "rz"] as const;

/**
 * One physical area of a scene (schema 3.0): a double page (one surface, twice the page's width), a cover, a wall, a
 * poster. Surfaces are free areas next to each other – no left / right, no front / back.
 */
export interface SurfaceData {
  /** Unique within its scene; an entry names it (`EntryData.surface`), a target's place too (`TargetPlace.surface`) */
  id: string;
  /** Optional: what is printed on it (`186–187`, `Cover`) – free text, never arithmetic */
  label?: string;
  widthMm: number;
  heightMm: number;
  /** Where it lies in the scene (`SurfacePlace`) */
  place: SurfacePlace;
}

/** Several scenes under one label (`Front matter`, a chapter, a room) – a scene names it (`SceneData.group`) */
export interface GroupData {
  id: string;
  label: string;
}

/**
 * A scene: one step of the experience (#307, `agents/SCENES.md`) – a double page, a cover, a wall, a room. Its place in
 * the experience is its position in `GameConfiguration.scenes`; nothing derives order from a number.
 */
export interface SceneData {
  /** Stable and invisible: links (`/scene/<id>`), progress and entries name it */
  id: string;
  /** What the reader's switcher and lists show for it – free text (`186–187`, `Cover`, `Studio wall`) */
  label: string;
  /** Optional: the id of the group the scene belongs to (`GameConfiguration.groups`) */
  group?: string;
  /** Its physical areas, in order – at least one */
  surfaces: SurfaceData[];
  /** Optional (schema 2.5): the post-processing passes over this scene's rendered image, in order; replaces the experience's (`[]` = none) */
  passes?: PassData[];
  /** Optional (schema 5.0, #334): the scene's rules – live while the scene is open (`./logic.ts`); its canvas never reaches a reader */
  logic?: LogicData;
}

export interface AssetData {
  id: string; // unique per scene, referenced by entities (`#id`)
  assetType: AssetType;
  src: string;
  /** Optional (schema 2.27), a model file only (glb / gltf): its maps in several sizes */
  textures?: TextureSetData[];
  /**
   * Optional (schema 4.3, #331), an image, video or audio file only: the medium in several quality levels (image px,
   * video height, audio kbps – `MEDIA_LEVELS`); `src` stays the file an older app loads
   */
  levels?: MediaLevelData[];
}

/**
 * An AR entity projected on a found target: a **tree of nodes** (content schema 2.0). The root node sits on the
 * target (1 unit = the target's width, origin in its centre); every node has a placement (`params`) relative to
 * its parent and may hold `children`. A 1.x entity is a tree of one node, so the app reads both.
 */
export interface EntityData {
  /** Optional, unique within the tree: the editor's selection */
  id?: string;
  /**
   * Optional (schema 2.12): the node's name for the people who make the book – the editor's tree shows it; a behaviour's
   * action names a node of its subtree by it (schema 2.19)
   */
  name?: string;
  /** A built-in type (ENTITY_TYPES) or a plugin's (schema 2.13) */
  type: NodeType;
  /** The node's own files (a model, a video, an image); empty for `group`, `text` and `light` (a plugin's type: its spec's `src`) */
  assets: AssetData[];
  /** Placement: position / rotation / scale (see ./placement.ts); a plugin node type's parameters (schema 2.13, `./node-types.ts`) */
  params?: Record<string, unknown>;
  filters?: FilterData[]; // video only, applied in order (see ./filters.ts)
  /**
   * Components applied to this node (schema 2.2 – `componentSpec`): the built-in `animation` on a model and `behaviour`
   * (schema 2.19, rules without code – `./behaviour.ts`), a plugin's component or material layer (schema 2.4)
   */
  components?: ComponentData[];
  /**
   * Optional (schema 2.17): a material a plugin registered (`registerMaterial`) that replaces the material of the node's
   * meshes – its type and values (`./materials.ts`); the node's material layers still add on top
   */
  material?: MaterialData;
  /** Child nodes, placed relative to this one */
  children?: EntityData[];
  /** `text` nodes: the line shown; `{{entry.title}}` and `{{experience.title}}` are replaced */
  text?: string;
  /** `text` nodes: the colour, `#rrggbb` (default white) */
  color?: string;
  /** `light` nodes (schema 2.6 – `LIGHT_PARAMS`): kind, colour, intensity, …; every value optional */
  light?: LightData;
}

/** Reference to a shared entity in `GameConfiguration.entities`. */
export interface EntityRefData {
  ref: string;
}

/**
 * Where a target is printed (schema 3.0): its box in mm from the top left corner of a surface of its entry's scene –
 * the editor's stage lays the target there; the reader does not need it.
 */
export interface TargetPlace {
  /** The surface it lies on, by id (`SurfaceData.id`) – required when the scene has several */
  surface?: string;
  x: number;
  y: number;
  width: number;
  height?: number;
}

export interface TargetData {
  id: string; // defaults to the entry id
  imageSrc: string;
  entity?: EntityData | EntityRefData;
  /** Optional: where it is printed (`TargetPlace`) – targets made in the editor carry it */
  place?: TargetPlace;
  /**
   * Optional (schema 2.26): `false` = switched off (#249) – the target keeps its values and its entry in the content, the
   * reader leaves both out (no image target, no entry in the list, not counted); none or `true` = in the experience
   */
  enabled?: boolean;
  /**
   * Optional (schema 4.1, #306): the image tracker's recognition data, made ahead from `imageSrc` (`osct targets prepare`)
   * – none = the reader makes it from `imageSrc` when the scene opens. The API drops it whenever `imageSrc` changes.
   */
  recognition?: TargetRecognitionData;
  /**
   * Optional (schema 4.2, #306): `true` = this target always uses its printed image, even when the experience says
   * `recognition: "prepared"` – the target keeps its `recognition`; none or `false` = the experience decides
   */
  recognitionOff?: boolean;
}

/**
 * A target's recognition data (#306) in the format of 8th Wall's image-target CLI, flat (`PLANAR`): the printed image in
 * a 3:4 portrait frame (landscape images turned 90° clockwise, the rest filled with the image's edge colour), grey,
 * 480 × 640 px. The frame's geometry follows from the printed image's size.
 */
export interface TargetRecognitionData {
  /** The grey 480 × 640 frame (a Library file) */
  imageSrc: string;
  /** Width and height in px of the printed image it was made from */
  width: number;
  height: number;
}

/** The target is in the experience: anything but `enabled: false` */
export const isTargetEnabled = (target: Pick<TargetData, "enabled">): boolean => target.enabled !== false;

/** The entry is in the reader's experience: no target, or a target that is switched on */
export const isEntryInExperience = (entry: Pick<EntryData, "target">): boolean => !entry.target || isTargetEnabled(entry.target);

export interface EntryData {
  id: string;
  category: EntryCategory;
  title: string;
  /** The id of the entry's scene (`SceneData.id`) – where the reader finds it */
  scene: string;
  /** Optional: the id of one of its scene's `surfaces` the entry lies on – only when the scene has several */
  surface?: string;
  /**
   * The text (schema 2.23: Markdown – paragraphs, *emphasis*, **strong**, [links](https://…) – with the experience's
   * variables in braces, `{hasKey}`; `../text/entry-text.ts` renders it). Plain text reads as before.
   */
  body: string;
  author?: string; // texts
  image?: string;
  /** Optional (schema 4.3, #331), with `image` only: the picture in several sizes (longest side 512 / 1024 / 2048 px) */
  imageLevels?: MediaLevelData[];
  /** Optional (schema 2.34, #289): the line under the picture ("Image: … by …"), plain text */
  caption?: string;
  media?: string; // links: external URL
  tags: string[];
  target?: TargetData;
}

export interface GameConfiguration {
  version: ConfigurationVersion;
  experience: ExperienceData;
  /** The scene the reader starts on (`SceneData.id`) */
  initialScene: string;
  /** The experience: its scenes in order */
  scenes: readonly SceneData[];
  entries: readonly EntryData[];
  entities: Readonly<Record<string, EntityData>>; // shared entities, referenced by `EntityRefData`
  /** The experience's own code (schema 2.4, optional): compiled plugins the engine loads before the first scene */
  plugins?: readonly PluginData[];
  /**
   * Optional (schema 2.31, #257): the subtitle files of the experience's videos, per language (`./subtitles.ts`) – read from the
   * files' names by the content build and the editor's API, never filled in by hand
   */
  subtitles?: readonly SubtitleData[];
  /**
   * Optional (schema 2.33, #292): the credit line of each file the experience uses that has one (`"Book Golem by TER-JAN /
   * DeviantArt"`), keyed by the file's path or URL exactly as the configuration names it – written in the editor's
   * Library, filled in by the editor's API when it assembles the experience, never by hand. The reader lists them per scene
   * on its image credits page
   */
  credits?: Readonly<Record<string, string>>;
  /** Optional: the groups scenes name (`SceneData.group`) – a label over several scenes */
  groups?: readonly GroupData[];
}
