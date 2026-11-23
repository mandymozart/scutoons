/**
 * The editor API's answers (`/api/editor/*` – server/api/src/Experiences.php, ExperienceTexts.php, ContentVersion.php, Assets.php,
 * Plugins.php, Removal.php, Export.php; docs/server.md "Editor data"), once for the editor app and the MCP server. Each type is what the PHP
 * sends: a field is optional only where the PHP leaves it out, `| null` where it sends null.
 * Not content: `shared/api/` is outside the content schema lock (RULES #10).
 */
import type { ExperienceAppearance } from "../types/appearance";
import type { ContentManifest } from "../types/content-manifest";
import type { EnvironmentData } from "../types/environment";
import type { ExperienceData, EntityData, EntryData, GameConfiguration, GroupData, LogicData, PassData, VariableDeclaration, RecognitionMode, SceneData, SurfaceData, TargetData } from "../types/game-config";
import type { AssetKind, SourceTypes } from "./rules";

// ── Experiences ──────────────────────────────────────────────────────────────────────────────────────────

/** Where an experience is live: an environment serving its build `build` since `since` */
export interface LiveIn {
  env: string;
  build: string | null;
  since: string | null;
}

/** What an environment serves (`experience` null when it cannot be told), or `error` when its content store cannot be read */
/** An environment where an experience's store cannot be read (`GET /editor/experiences`, `environments`) */
export interface EnvironmentUnreadable {
  env: string;
  error: string;
}

/** An experience of the list and of the tree (`Experiences::summary`) */
export interface ExperienceSummary {
  id: string;
  title: string;
  /** The configuration's `experience` as imported and edited since: its environment, its passes, its appearance, its
   * printed book (`experience.book` – the author is `experience.book?.author`) */
  experience: ExperienceData;
  /** The scene the experience opens on (`initialScene`) – it cannot be removed */
  initialScene: string;
  /** The content schema `MAJOR.MINOR` of the import */
  schema: string;
  /** Where the content's `/assets/content/…` paths resolve (the layout, the cover); "" = the draft's origin */
  assetsBase: string;
  /** The public folder of the experience's uploads (`…/assets/<experience>/`, the Library's files lie below it); "" = the instance has no public URL for them */
  libraryBase: string;
  /** `version.hash` of the imported configuration, null when it had none */
  importedHash: string | null;
  /** How many scenes it has */
  scenes: number;
  createdAt: string;
  updatedAt: string;
  lastPublish: { env: string; build: string; at: string } | null;
  /** The environments serving this experience – empty: draft only */
  live: LiveIn[];
}

/** `GET /editor/experiences`: the experiences, and the environments where a store of theirs cannot be read */
export interface ExperienceList {
  experiences: ExperienceSummary[];
  environments: EnvironmentUnreadable[];
}

/** An entry in the tree; `category` as the content holds it */
export interface TreeEntry {
  id: string;
  category: string;
  title: string;
  /** Its scene (`SceneData.id`) */
  scene: string;
  /** The surface of its scene it lies on – only when it names one */
  surface?: string;
  /** Its target (the entity inline or `{ ref }`, its `place`), null when it has none */
  target: TargetData | null;
}

/** A scene in the tree: the scene as the content holds it (schema 3.0) with the entries on it */
export type TreeScene = SceneData & { entries: TreeEntry[] };

/** `GET /editor/experiences/{id}`: the experience, its scenes in order with their entries, the groups */
export interface ExperienceTree {
  experience: ExperienceSummary;
  scenes: TreeScene[];
  groups: GroupData[];
}

/** `GET /editor/experiences/{id}/draft/latest.json`: the draft's manifest – its build now (what a publish would write) */
export type DraftManifest = Required<ContentManifest>;

// ── Environments and publishing ──────────────────────────────────────────────────────────────────────

/** An environment as `GET /editor/environments` describes it: its backend, whether this instance publishes there */
export interface EnvironmentInfo {
  env: string;
  /** `cdn` – the CDN's storage zone; `server` – the directory store this API serves */
  store: "cdn" | "server";
  /** Only the instance's own environment */
  publish: boolean;
  /** Where readers load it: the environment base (`<pull zone>/<title>/<env>`, this API's `/content/<env>`) – an experience's store is `<url>/<experience>` (#87) */
  url: string | null;
  /** Wherever the directory store is the publish target: what that means */
  warning: string | null;
}

/** `GET /editor/environments`: this instance's environment (the only one it publishes to) and every environment's backend */
export interface EnvironmentsInfo {
  env: string;
  environments: EnvironmentInfo[];
}

/** `POST /editor/experiences/{id}/publish` */
export interface PublishResult {
  env: string;
  /** Where it went: the CDN or this server's directory store */
  store: "cdn" | "server";
  build: string;
  hash: string;
  publishedAt: string;
  /**
   * The build's export archive (#322, `ExportArchive`): `exports/<build>` in the experience's store – `written` false when
   * that build's archive was there already, or with `error` when writing it failed (the publish itself went through)
   */
  export: { path: string; written: boolean; error?: string };
}

/**
 * A publish as a job (#345, server/api/src/PublishJob.php): `POST …/publish` answers 200 with the outcome and `job` when it
 * ended within the server's step, else 202 `{ job }` – then `GET …/publish/{job}` reports it (and writes the next part
 * where nothing runs on after an answer); `GET …/publish` = the experience's newest job (`{ job: null }`: none kept).
 */
export interface PublishJob {
  id: string;
  experience: string;
  env: string;
  status: "running" | "done" | "failed";
  /** version (the build's folder) → switch (versions.json, latest.json) → archive (the export archive) → done */
  phase: "version" | "switch" | "archive" | "done";
  build: string;
  hash: string;
  /** latest.json names the build – readers get it; the export archive may still be written */
  live: boolean;
  /** Files written or found on the store already, of `total` (the two of the switch included) */
  done: number;
  total: number;
  /** Files the store held already (not written again) */
  skipped: number;
  /** The files of the version folder (`build`) and of the export archive (`archive`): total = build + 2 (the switch) + archive;
   * absent on a job an older server started */
  parts?: { build: number; archive: number };
  startedAt: string;
  updatedAt: string;
  /** done: what the publish answers */
  result?: PublishResult;
  /** failed: why (`abandoned`: no one continued it and a newer publish took its place) */
  error?: { code: string; message: string; status: number };
}

/** `POST …/publish` done in time: the outcome with its job; still running (202): the job alone */
export type PublishStarted = (PublishResult & { job: PublishJob }) | { job: PublishJob };

/** `POST /editor/experiences/{id}/versions/{build}/activate` (#348): the build that is live again, and the one it replaced */
export interface VersionActivated {
  env: string;
  build: string;
  hash: string;
  activatedAt: string;
  previous: string | null;
  version: { publishedAt: string; note: string };
}

/** A change of `GET /editor/recent` (#348): across the instance's experiences, with the scene it is on (null: none) */
export interface RecentChange {
  experience: string;
  experienceTitle: string;
  kind: string;
  ref: string;
  scene: string | null;
  sceneLabel: string | null;
  /** The entry's title for an entry's or a target's change */
  entry: string | null;
  user: string | null;
  token: string | null;
  at: string;
}

/**
 * The export file (#348, `GET /editor/experiences/{id}/export/file`): an import body as it is – `configuration`, `files`,
 * `assetsBase` (a published build's folder) and the Code tab `sources` – with what it is of
 */
export interface ExportDownload {
  format: "osct-export";
  formatVersion: number;
  experience: string;
  title: string;
  build: string;
  appVersion: string;
  exportedAt: string;
  source: { kind: "draft" } | { kind: "published"; env: string; publishedAt: string; publishedBy: string | null };
  configuration: Record<string, unknown>;
  files: Record<string, unknown>;
  assetsBase?: string;
  sources: { id: string; source: string }[];
}

/** A row of the publish log (`GET /editor/experiences/{id}/versions`, newest first) */
export interface PublishedVersion {
  id: string;
  env: string;
  build: string;
  hash: string;
  /** The e-mail address of who published, null when the account is gone */
  publishedBy: string | null;
  publishedAt: string;
  note: string | null;
}

// ── Export (#322) ────────────────────────────────────────────────────────────────────────────────────

/**
 * A file an export names (`Export::files`): its URL as the configuration names it, its content path (`/assets/content/…`,
 * null for a Library upload) and what is known of it. `origin`: `upload` – a Library file (size and full `sha256` from its
 * asset row); `import` – a file of the imported build (size and `v` from its files.json, no full sha256); `version` – a Code
 * tab plugin a publish put into the version's folder (`url` in that environment, nothing else known); `unknown` – a value
 * that looks like a file and is none of these (nothing known, the URL as written).
 */
export interface ExportFile {
  url: string;
  path: string | null;
  origin: "upload" | "import" | "version" | "unknown";
  size: number | null;
  sha256: string | null;
  /** The first 12 hex digits of its sha256 (files.json's `v`) */
  v: string | null;
}

/** `GET /editor/experiences/{id}/export[?build=]`: everything an export of the experience needs (docs/server.md "Export") */
export interface ExperienceExport {
  experience: string;
  /** The draft, or the published version `build` names – its environment, when and by whom */
  source: { kind: "draft" } | { kind: "published"; env: string; publishedAt: string; publishedBy: string | null };
  build: string;
  hash: string;
  /** The content schema `MAJOR.MINOR` of the configuration ("" when a published one does not say) */
  schema: string;
  /** This server's app version */
  appVersion: string;
  exportedAt: string;
  configuration: GameConfiguration;
  /** Every file the configuration names, in the order it names them, then the layout's */
  files: ExportFile[];
  /** How many of `files` are `unknown` */
  unknown: number;
  /** The print layout (`layout/layout.json` and its sheets – URLs, all in `files` too), null when the import had none */
  layout: { file: string; files: string[] } | null;
  /** The Code tab's sources – always the current ones */
  plugins: PluginSource[];
  /** Every credit line of the Library, by file URL */
  credits: Record<string, string>;
}

/**
 * `exports/<build>/export.json` of an export archive (#322, docs/content.md "Export archive"): written by a publish
 * (`ExportArchive`) or `osct content export --archive`. The media are not copied: every file the archive's YAML names is
 * listed with its path in the folder and the immutable URL it is read from; `builds` are the content builds those URLs lie
 * in (and the archived build itself) – what `osct content purge` keeps for the newest export of an experience.
 */
export interface ExportArchiveRecord {
  archive: true;
  experience: string;
  /** The API that wrote it, or the CLI's API URL */
  instance: string;
  title: string;
  env: string;
  /** Never a person: the archive is readable through the pull zone */
  source: ExperienceExport["source"];
  build: string;
  hash: string;
  schema: string;
  appVersion: string;
  writtenBy: "publish" | "cli";
  exportedAt: string;
  files: { path: string; url: string; origin: ExportFile["origin"]; sha256: string | null; v: string | null; size: number | null }[];
  builds: { env: string; build: string }[];
  problems: string[];
}

/** A row of the change log (`GET /editor/experiences/{id}/changes`, newest first); `token` = the AI connection's name, else null */
export interface Change {
  id: string;
  kind: string;
  ref: string;
  before: unknown;
  after: unknown;
  user: string | null;
  token: string | null;
  createdAt: string;
  /** The change set a revert wrote (#312): the changes of one revert share it; null for any other save */
  set?: string | null;
  /** The change this one took back (a revert), else null */
  reverts?: string | null;
  /** The delivery that wrote it (#322, `DeliveryResult.delivery.id`), else null */
  delivery?: string | null;
  /** Why the change cannot be reverted (a creation, a removal, a moved entry, a plugin source …); null when it can */
  notRevertable?: string | null;
}

/** One change of a revert's plan (#312): what happens to it */
export interface RevertItem {
  id: string;
  kind: string;
  ref: string;
  user: string | null;
  token: string | null;
  createdAt: string;
  /** A later change of the same thing, reverted with the chosen ones (its edit would be lost otherwise) */
  later: boolean;
  /** Made by someone else */
  other: boolean;
  /** `revert` – its before is written back; `conflict` – its thing no longer holds what it wrote (left as it is); `not-revertable` */
  status: "revert" | "conflict" | "not-revertable";
  reason?: string;
}

/**
 * `POST /editor/experiences/{id}/changes/{cid}/revert`, `…/changes/revert { ids }`, `…/changes/revert-to { after }` – without
 * `apply: true` the plan (nothing written), with it the plan written: `written` the new changes' ids, `set` their set
 */
export interface RevertResult {
  plan: {
    changes: RevertItem[];
    /** The change ids asked for (the rest are later changes of the same things) */
    chosen: string[];
    counts: { revert: number; conflict: number; notRevertable: number };
    /** How many later changes of the same things are reverted with them */
    later: number;
    /** The e-mail addresses of other people whose changes are reverted or in the way */
    others: string[];
  };
  applied: boolean;
  set: string | null;
  written: string[];
}

// ── Import ───────────────────────────────────────────────────────────────────────────────────────────

/** Where an import's configuration came from: the instance's CDN, or a configuration in the body */
export type ImportSourceKind = "cdn" | "configuration";

/** What an import recorded – the build the draft is based on (`Experiences::basedOn`); a field null where nothing told */
export interface ImportedBuild {
  build: string | null;
  hash: string | null;
  date: string | null;
  appVersion: string | null;
  schema: string | null;
  source: ImportSourceKind | null;
  from: string | null;
  importedAt: string | null;
  importedBy: string | null;
}

/** What an import brings in (`Experiences::incoming`): the experience, its build and where it comes from */
export interface IncomingBuild {
  experience: string;
  title: string;
  build: string | null;
  hash: string | null;
  date: string | null;
  appVersion: string | null;
  schema: string;
  source: ImportSourceKind;
  from: string | null;
}

/** The rows a configuration holds */
export interface ImportCounts {
  scenes: number;
  entries: number;
  targets: number;
  entities: number;
  /** The Code tab sources an import's `sources` (an export file) carries – only when it carries any (#348) */
  plugins?: number;
}

/** `details.comparison` of a 409 confirm-import, and a preview's `existing`: what comes in against the draft */
export interface ImportComparison {
  experience: { id: string; title: string };
  incoming: IncomingBuild;
  draft: {
    basedOn: ImportedBuild | null;
    build: string;
    edits: number;
    editsSince: string | null;
    recentEdits: { kind: string; ref: string; user: string | null; token: string | null; at: string }[];
    live: { env: string; build: string | null; current: boolean }[];
  };
  same: boolean;
  /**
   * What comes in against what the draft holds (#348; null in a preview without a configuration): scenes, entries,
   * targets, shared entities and Code tab sources each side, and the draft's sources the import's `sources` replace
   */
  counts?: { incoming: Required<ImportCounts>; draft: Required<ImportCounts>; sourcesReplaced: string[] } | null;
}

/**
 * `POST /editor/import/preview`: what an import body would bring in, nothing imported – `existing` is the comparison the
 * import would answer (null: a new experience); `contentBase` / `manifest` the CDN's content base and its latest.json
 */
export interface ImportPreview {
  source: ImportSourceKind;
  contentBase: string | null;
  manifest: { url: string; build: string; activatedAt: string | null } | null;
  incoming: IncomingBuild;
  counts: ImportCounts;
  existing: ImportComparison | null;
}

/** `POST /editor/experiences/{id}/import`: the experience, what came in, the draft's build now and the build it is based on */
export interface ImportResult {
  experience: ExperienceSummary;
  counts: ImportCounts;
  draft: { build: string; hash: string };
  imported: ImportedBuild | null;
}

/** `details.confirmation` of a two-step write's first 409 (`Confirmation::issue`): the id to send back, until `expiresAt` */
export interface Confirmation {
  id: string;
  expiresAt: string;
}

// ── Rows: entities, targets, entries, the experience's passes, appearance and environment ─────────────

/** `GET /editor/experiences/{id}/entities`: the shared entities by ref */
export interface SharedEntities {
  entities: Record<string, EntityData>;
}

/** `PUT /editor/experiences/{id}/entities/{ref}` */
export interface EntitySaved {
  ref: string;
  entity: EntityData;
}

/** `PUT /editor/experiences/{id}/targets/{targetId}`: the whole target after the change */
export interface TargetSaved {
  target: TargetData;
}

/** `POST /editor/experiences/{id}/entries` → 201: the new entry with its target (none when it was made without one), and its scene */
export interface EntryAdded {
  entry: { id: string; category: string; title: string; scene: string; surface?: string; body: string; caption?: string; tags: string[]; target?: TargetData };
  scene: string;
}

/** An entry as the content holds it, without its target (`ExperienceTexts`): every field as written – an image's path or URL unresolved */
export type EntryFields = Omit<EntryData, "target">;

/** One entry of `GET /editor/experiences/{id}/entries`: its fields (its `scene` among them) and its target's id and image */
export type EditorEntry = EntryFields & { target: { id: string; imageSrc: string } | null };

/** `GET /editor/experiences/{id}/entries`: every entry in the experience's order */
export interface EntryList {
  entries: EditorEntry[];
}

/** `PUT /editor/experiences/{id}/entries/{entryId}`: the entry after the change, its scene now and before */
export interface EntrySaved {
  entry: EntryFields;
  scene: string;
  previousScene: string;
}

/**
 * `PUT /editor/experiences/{id}`'s body (#320): the fields sent change. `book` changes the printed book's fields sent
 * (`publisher: null` or "" removes it); `book: null` removes the printed book; a new one needs its `author`.
 */
export interface ExperienceUpdate {
  title?: string;
  book?: { author?: string; publisher?: string | null } | null;
  /** Schema 4.2 (#306): how the reader gets its targets' recognition data; `"original"` or null = the default (stored as none) */
  recognition?: RecognitionMode | null;
}

/** `PUT /editor/experiences/{id}`: the experience after the change (its title, its printed book and the rest it holds) */
export interface ExperienceSaved {
  experience: ExperienceData;
}

// ── Scenes (#307, schema 3.0 – docs/server.md "Scenes") ─────────────────────────────────────────────

/**
 * The presets a new scene starts from (`Scenes::PRESETS`, `POST …/scenes { preset }`): `double` – one surface twice the
 * page's width, `single` – one page, `wall` – a wall standing ahead (`rx` 90). A cover is a `single` scene first in the order.
 */
export const SCENE_PRESETS = ["double", "single", "wall"] as const;
export type ScenePreset = (typeof SCENE_PRESETS)[number];
/** A page's size (`Scenes::PAGE_MM`): a double page is twice its width */
export const SCENE_PAGE_MM = { width: 230, height: 320 } as const;
/** A wall's size (`Scenes::WALL_MM`) */
export const SCENE_WALL_MM = { width: 2000, height: 1500 } as const;
/** The most surfaces one scene has (`Scenes::SURFACES_MAX`) */
export const SCENE_SURFACES_MAX = 32;
/**
 * The targets a group's loading window holds at once (#307 phase 3a): the editor's Scenes panel warns above it. A first
 * guess – a few scenes at the engine's 10 targets each (RULES #3, `prepareTargets` keeps ±2 scenes ready).
 * TODO(#300): replace with the window measured on the phone.
 */
export const GROUP_WINDOW_TARGETS = 20;

/** How a surface's reference image fills it (#314, server `SurfaceReferences::FITS`): cut to cover, fit inside, stretched */
export const SURFACE_REFERENCE_FITS = ["cover", "contain", "stretch"] as const;
export type SurfaceReferenceFit = (typeof SURFACE_REFERENCE_FITS)[number];

/**
 * A surface's reference image (#314): an image of the experience's Library the editor's stage shows on the surface – editor-only,
 * never in the content. `url` = a Library URL or an imported file's content path (`/assets/content/…`).
 */
export interface SurfaceReference {
  url: string;
  name: string;
  fit: SurfaceReferenceFit;
}

/** A surface as the scenes overview shows it: the content's `SurfaceData` and its reference image (null: none) */
export type SurfaceSummary = SurfaceData & { reference: SurfaceReference | null };

/**
 * `PUT /editor/experiences/{id}/scenes/{sceneId}/surfaces/{surfaceId}/reference`: `url` sets or replaces (null removes);
 * `fit` / `name` without `url` change the present one (a replace keeps its fit unless `fit` is sent)
 */
export interface SurfaceReferenceRequest {
  url?: string | null;
  fit?: SurfaceReferenceFit;
  name?: string;
}

/** The answer of `PUT …/surfaces/{surfaceId}/reference`: the reference now (null: none), whether anything changed */
export interface SurfaceReferenceSaved {
  scene: string;
  surface: string;
  reference: SurfaceReference | null;
  changed: boolean;
}

/**
 * A scene of `GET /editor/experiences/{id}/scenes`: the content's `SceneData`, its place in the order, its surfaces each with
 * its reference image, and how many entries and targets lie on it
 */
export type SceneSummary = Omit<SceneData, "surfaces"> & {
  position: number;
  surfaces: SurfaceSummary[];
  entries: number;
  targets: number;
};

/** `GET /editor/experiences/{id}/scenes`: the scenes in order and the groups with their scenes */
export interface SceneList {
  scenes: SceneSummary[];
  groups: (GroupData & { scenes: string[] })[];
}

/**
 * A surface a request sends (`POST …/scenes { surfaces }`, `PUT …/scenes/{id} { surfaces }`): `SurfaceData` whose `place`
 * may be left out (the server lays every surface without one in a row, as the content build does) and whose rotations
 * default to 0
 */
export type SurfaceInput = Omit<SurfaceData, "place"> & { place?: { x: number; y: number; z: number; rx?: number; ry?: number; rz?: number } };

/** `POST /editor/experiences/{id}/scenes` { preset?, after? | before? | position?, id?, label?, group?, surfaces? } → 201 */
export interface SceneAdded {
  scene: SceneData;
  position: number;
}

/**
 * `PUT /editor/experiences/{id}/scenes/{sceneId}` { label?, group?, surfaces?, passes?, apply? }: the scene after the change; the
 * entries whose surface went (they stay on the scene without one) and the targets whose surface went (their `place` goes)
 */
export interface SceneSaved {
  scene: SceneData;
  cleared: { entries: string[]; targets: string[] };
}

/** A new order (a move, a group): every id in order, the picked scenes that move, group changes */
export interface SceneOrderPlan {
  order: string[];
  moved: string[];
  groups: { scene: string; from: string | null; to: string | null }[];
}

/** `POST /editor/experiences/{id}/scenes/move`: the plan, and whether it was written (`apply: true`) */
export interface ScenesMoved {
  plan: SceneOrderPlan;
  applied: boolean;
}

/** `POST /editor/experiences/{id}/groups`: the group and its plan, and whether it was written */
export interface GroupPlanned {
  plan: SceneOrderPlan & { group: GroupData };
  applied: boolean;
}

/** `PUT /editor/experiences/{id}/groups/{gid}` */
export interface GroupSaved {
  group: GroupData;
}

/** `DELETE /editor/experiences/{id}/groups/{gid}`: the group that went and its scenes (in place, without a group now) */
export interface Ungrouped {
  ungrouped: string;
  scenes: string[];
}

/** `PUT /editor/experiences/{id}/passes`: the experience's post-processing, null when removed */
export interface ExperiencePassesSaved {
  passes: PassData[] | null;
}

/** `PUT /editor/experiences/{id}/appearance`: the experience's looks by moment, null when removed (the default look plays everywhere) */
export interface ExperienceAppearanceSaved {
  appearance: ExperienceAppearance | null;
}

/**
 * `PUT /editor/experiences/{id}/logic` { logic: LogicData | null } (content schema 5.0, #334): the experience's rules with
 * their canvas, null when removed. A scene's rules go with `PUT …/scenes/{sid}` { logic } (the answer's `scene` carries
 * them). Change kind `logic`. 400 `invalid-logic` (a scene's: `invalid-scene`) names a shape the API refuses.
 */
export interface ExperienceLogicSaved {
  logic: LogicData | null;
}

/** `PUT /editor/experiences/{id}/variables` (content schema 5.0, B7): the rules' typed variables, null when removed. 400 `invalid-variables` */
export interface ExperienceVariablesSaved {
  variables: Record<string, VariableDeclaration> | null;
}

/** `PUT /editor/experiences/{id}/environment`: the experience's environment, null when removed (the default applies) */
export interface EnvironmentSaved {
  environment: EnvironmentData | null;
}

// ── Removing a scene or an entry ─────────────────────────────────────────────────────────────────

/** A Library file a removal touches: used only by the removed rows, or (`usedBy`) elsewhere too */
export interface RemovalFile {
  id: string;
  name: string;
  kind: AssetKind;
  size: number;
  url: string;
  usedBy?: Usage[];
}

/** What every removal takes (`Removal::impact`) */
interface RemovalTakes {
  experience: string;
  entries: { id: string; title: string; scene: string | null; category: string }[];
  targets: { id: string; entry: string; imageSrc: string }[];
  /** The entities that go: a shared one with its `ref`, a target's own with `ref: null` */
  entities: { ref: string | null; type: string | null; shared: boolean; targets: string[] }[];
  /** Shared entities that stay, because `usedBy` targets elsewhere use them */
  keptEntities: { ref: string; type: string; targets: string[]; usedBy: string[] }[];
  files: { onlyHere: RemovalFile[]; elsewhere: RemovalFile[] };
  counts: { entries: number; targets: number; entities: number; keptEntities: number; filesOnlyHere: number; filesElsewhere: number };
}

/** `details.impact` of a 409 confirm-delete (docs/server.md "Removing a scene or an entry") */
export type RemovalImpact = RemovalTakes &
  (
    | { kind: "scene"; scene: { id: string; label: string } }
    | { kind: "entry"; entry: { id: string; title: string; category: string; scene: string | null } }
  );

/** What a confirmed removal took (`files` only with `deleteFiles`) */
export interface RemovalResult {
  deleted: { scene: string | null; entries: string[]; targets: string[]; entities: string[]; files: string[] };
}

// ── Assets ───────────────────────────────────────────────────────────────────────────────────────────

/** A Library file (`GET /editor/experiences/{id}/assets`): an upload, or a file of the content import (`imported`) */
export interface Asset {
  id: string;
  name: string;
  /** `shader`: a source file a plugin loads by its URL (`api.loadText`) */
  kind: AssetKind;
  mime: string;
  size: number;
  /** The picture's size in pixels (images, videos), null when the file does not tell */
  width: number | null;
  height: number | null;
  /** A video's duration in ms, null for other kinds or when the file does not tell */
  durationMs: number | null;
  version: number;
  /** The absolute URL nodes and targets name */
  url: string;
  usages: number;
  /** A file of the content import (`files.json`), not an upload: read-only, no replace or delete */
  imported: boolean;
  /**
   * The stored file's sha256 (hex) – what an upload is deduplicated by; `osct delivery import` skips a file the Library
   * already holds (#322). Null for an imported file
   */
  sha256?: string | null;
  /**
   * The picture credits (#277): one free text line, as the book prints them ("Book Golem by TER-JAN / DeviantArt"); null =
   * none yet. Kept per URL – imported files carry one too.
   */
  credits: string | null;
  /**
   * Where the file is used first (#298 phase 3, `AssetNames`) – the Library groups by it: the scene of its first use in
   * the scenes' order (null: experience-wide – the environment, a plugin, a shared entity no target shows), that order (−1
   * experience-wide) and how many uses follow ("+N more"); null for an unused file
   */
  place?: LibraryPlace | null;
  /** An imported file listed under a name given by its use (`asset_names`), not its own */
  named?: boolean;
  createdAt: string;
  updatedAt: string;
}

/** A file's first use (`Asset.place`) */
export interface LibraryPlace {
  scene: string | null;
  order: number;
  more: number;
}

/**
 * `POST /editor/experiences/{id}/assets/names { apply? }` (#298 phase 3): the names the Library's files take by their use –
 * `p118-1_target.webp`, `p118-1_image.webp`, `<scene>_<surface>.webp` (a reference image), `<entity>_<asset>.glb` … The name only: every URL stays.
 * Without `apply: true` the plan; with it the plan is written (`applied`: something was renamed).
 */
export interface AssetNamesPlan {
  plan: {
    renames: AssetRename[];
    /** The files of the Library, the used ones, the unused ones (they keep their names) */
    files: number;
    used: number;
    unused: number;
  };
  applied: boolean;
}

/** One file of the plan: its name now and by its first use; `more` = its further uses */
export interface AssetRename {
  id: string;
  url: string;
  imported: boolean;
  from: string;
  to: string;
  use: { kind: string; ref: string; scene: string | null };
  more: number;
}

/** Where a file or a plugin is used: a row of the draft (`entity castle`), the experience, or a plugin source that loads the file */
export interface Usage {
  kind: string;
  ref: string;
}

/** `GET /editor/experiences/{id}/assets[?kind=]` */
export interface AssetList {
  assets: Asset[];
}

/**
 * `POST /editor/experiences/{id}/assets[?fit=<use>]` → 201 (or 200: the same bytes are an asset already – it takes this upload's
 * name). `converted`: the picture was stored under the image rule (#298, `shared/api/images.ts`) – as WebP named
 * `<name>.webp`, scaled down to the use's longest side; false for every other file, and for a picture this instance
 * stores as it came (no GD, unreadable, animated, a WebP within the rule).
 */
export interface AssetUploaded {
  asset: Asset;
  converted: boolean;
}

/** The most files one `PUT /editor/experiences/{id}/assets/credits` sets (`Assets::CREDITS_BATCH_MAX`) */
export const CREDITS_BATCH_MAX = 500;
/** The longest credits line (`Assets::CREDITS_MAX`) */
export const CREDITS_MAX = 1000;

/** `PUT /editor/experiences/{id}/assets/credits` – a file's credits line by its URL; empty or null removes it (#277) */
export interface AssetCreditsChange {
  items: { url: string; credits: string | null }[];
}

/** The answer: how many files, and each one's line now (null: none) */
export interface AssetCreditsSaved {
  saved: number;
  credits: Record<string, string | null>;
}

/**
 * `PUT /editor/experiences/{id}/assets/{assetId}` `{ name }` (#330): one file's name by hand – an upload's or an imported
 * file's (`import-…`); the URL stays. The name as stored (white space folded, the extension kept) and whether it changed.
 */
export interface AssetRenamed {
  asset: { id: string; url: string; name: string; imported: boolean };
  changed: boolean;
}

/** `PUT /editor/experiences/{id}/assets/{assetId}/file[?fit=<use>]`: the new version, how many rows now name it, and whether the image rule converted it */
export interface AssetReplaced {
  asset: Asset;
  relinked: number;
  converted: boolean;
}

/** `GET /editor/experiences/{id}/assets/{assetId}/usages` */
export interface AssetUsages {
  usages: Usage[];
  asset: Asset;
}

// ── Plugin sources (the Code tab) ────────────────────────────────────────────────────────────────────

/** A plugin source in the list (`GET /editor/experiences/{id}/plugins`) */
export interface PluginSummary {
  id: string;
  size: number;
  /** The draft's rows that use a type it registers */
  usages: number;
  updatedAt: string;
  updatedBy: string | null;
}

/** One plugin source (`GET`, `PUT`, `POST …/plugins`, `PUT …/file` – `{ plugin }`): its text, without the list's `usages` */
export interface PluginSource {
  id: string;
  source: string;
  size: number;
  updatedAt: string;
  updatedBy: string | null;
}

/** `GET /editor/experiences/{id}/plugins/{pluginId}/usages`: where the draft uses what it registers, and what that is */
export interface PluginUsages {
  usages: Usage[];
  registers: SourceTypes;
}
