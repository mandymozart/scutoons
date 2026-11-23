/**
 * Read access to the game configuration (`game.config.json`): the data the content build
 * (`scripts/`) generates from the authored content.
 *
 * The only module that reads it (RULES #14). `loadGameConfiguration()` loads it once before the app starts:
 * from a content base URL (`<base>/latest.json` names the current version, `<base>/<build>/` holds it and never
 * changes – the CDN, docs/content-cdn.md, or the editor's draft endpoint), else the configuration the app bundles
 * (`options.bundled` – the reader's dev server and tests). The engine knows no `import.meta.env` and no i18n: the
 * app passes the base URL, the bundled copy and the texts of the startup errors.
 * On load it checks that this app can read the content (content schema MAJOR – RULES #10), drops what content of a
 * newer MINOR has that this app does not know (`./forward-compatible.ts` – the list stays for `getSkippedFeatures()`),
 * checks the JSON against the contract (`assertGameConfiguration`), maps `*Data` to the app model (`Scene`, `Entry`,
 * `Target`) and builds in-memory indexes. Content file paths (`/assets/content/…`) become URLs of the loaded version, so the
 * rest of the app never knows where the content comes from.
 */
import { assertGameConfiguration, GameConfigurationError, type GuardOptions, isEntityRef } from '@shared/guards/game-config';
import { CONTENT_SCHEMA_VERSION, OLDEST_READABLE_SCHEMA_MAJOR, parseContentSchema } from '@shared/types/game-config';
import { pickSubtitleTrack } from '@shared/types/subtitles';
import { contentRuleLists, type VariableInfo, variableTypes } from '@shared/types/variables';
import type { MediaLevelData } from '@shared/types/media-levels';
import { currentFeatures, describeSkipped, type KnownFeatures, skipUnknownFeatures, type SkippedFeature } from './forward-compatible';
import { entryPlaceLabel } from './scenes';
import { type ContentFiles, type ContentManifest, isContentFiles, isContentManifest } from '@shared/types/content-manifest';
import {
  type AssetData,
  type ExperienceData,
  type ConfigurationVersion,
  type EntityData,
  type EntityRefData,
  type Entry,
  type GroupData,
  ErrorCode,
  type ErrorInfo,
  type GameConfiguration,
  isEntryInExperience,
  type PassData,
  type PluginData,
  type Scene,
  type Target,
} from "../types/index";

/** Used until the configuration is loaded, and when it is invalid, so the app can show the error */
const EMPTY_CONFIGURATION: GameConfiguration = {
  version: { version: '0.0.0', timestamp: '', schema: CONTENT_SCHEMA_VERSION },
  experience: { id: '', title: '' },
  initialScene: '',
  scenes: [],
  entries: [],
  entities: {},
};

/** Path prefix of content files in the configuration and in the app's public folder */
export const CONTENT_PATH = '/assets/content/';
/** Fetching the configuration: a slow CDN must not keep the splash forever */
const LOAD_TIMEOUT_MS = 20000;

/**
 * Runtime error handling: the guard's `GameConfigurationError` (every problem with its path, same
 * as the build reports) goes to the console; the app gets an `ErrorInfo` with an `ErrorCode`.
 * `main.ts` checks `getConfigurationError()` before starting and shows a critical error instead.
 */
let configurationError: ErrorInfo | null = null;

/** Texts of the startup errors (`ErrorInfo.msg`) – the app translates them */
export interface LoadTexts {
  contentNewer: string;
  contentOlder: string;
  contentNotLoaded: string;
}

const DEFAULT_TEXTS: LoadTexts = {
  contentNewer: 'This content needs a newer version of the app.',
  contentOlder: 'This content needs a newer build.',
  contentNotLoaded: 'The content could not be loaded.',
};

export {
  currentFeatures, describeSkipped, isNewerMinor, type KnownFeatures, skipUnknownFeatures, type SkippedFeature, type SkippedKind,
} from './forward-compatible';

/**
 * Can this app read the content? The content schema (`MAJOR.MINOR`, shared/) is the only rule (RULES #10): a newer
 * MAJOR is refused (the app needs an update), so is one below `OLDEST_READABLE_SCHEMA_MAJOR` (the content needs a
 * newer build); every MAJOR in between loads as it is, and any MINOR of the app's MAJOR – a newer one without what
 * this app does not know (`skipUnknownFeatures`). The app version the content was built with is information only.
 * `appSchema` is the schema this app reads (a test passes an older one).
 */
export const checkContentCompatibility = (
  version: ConfigurationVersion,
  texts: LoadTexts = DEFAULT_TEXTS,
  appSchema: string = CONTENT_SCHEMA_VERSION,
): ErrorInfo | null => {
  const app = parseContentSchema(appSchema);
  const content = parseContentSchema(version.schema);
  // Older content is read as it is, down to the oldest MAJOR the contract still knows
  if (app && content && content.major <= app.major && content.major >= OLDEST_READABLE_SCHEMA_MAJOR) {
    if (content.major !== app.major || content.minor !== app.minor) {
      const newer = content.major === app.major && content.minor > app.minor;
      console.info(`[game-config] Content schema ${version.schema}, this app reads ${appSchema} – compatible${newer ? ', what this app does not know is skipped' : ''}.`);
    }
    return null;
  }
  const newer = !!app && !!content && content.major > app.major;
  return {
    code: ErrorCode.NOT_SUPPORTED,
    msg: newer ? texts.contentNewer : texts.contentOlder,
    type: 'critical',
    details: [
      `Content schema ${version.schema} (built with app ${version.version}), this app reads schema ${appSchema}: ` +
        (newer ? 'update the app.' : 'the content needs a newer build.'),
    ],
  };
};

const fetchWithTimeout = async (fetchFn: typeof fetch, url: string, init: RequestInit = {}): Promise<Response> => {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), LOAD_TIMEOUT_MS);
  try {
    const response = await fetchFn(url, { ...init, signal: controller.signal });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText} (${url})`);
    return response;
  } finally {
    clearTimeout(timer);
  }
};

/**
 * The current content version on the CDN. latest.json is the only file that changes: revalidated by the browser
 * and asked for with a value that changes once a minute (`?t=`, for edges that vary their cache by query string).
 * The version folder never changes – its files are cached like any immutable file. files.json (sizes for the
 * whole-experience download) is optional.
 */
const fetchFromContentBase = async (
  baseUrl: string,
  fetchFn: typeof fetch,
): Promise<{ raw: unknown; files: ContentFiles; versionUrl: string; manifest: ContentManifest }> => {
  const manifest: unknown = await (
    await fetchWithTimeout(fetchFn, `${baseUrl}/latest.json?t=${Math.floor(Date.now() / 60000)}`, { cache: 'no-cache' })
  ).json();
  if (!isContentManifest(manifest)) throw new Error(`${baseUrl}/latest.json does not name a content version`);
  const versionUrl = `${baseUrl}/${manifest.build}`;
  const [raw, files] = await Promise.all([
    fetchWithTimeout(fetchFn, `${versionUrl}/game.config.json`).then(r => r.json() as Promise<unknown>),
    fetchWithTimeout(fetchFn, `${versionUrl}/files.json`)
      .then(r => r.json() as Promise<unknown>)
      .catch(error => {
        console.warn(`[game-config] ${versionUrl}/files.json not loaded (${error instanceof Error ? error.message : error})`);
        return null;
      }),
  ]);
  return { raw, files: isContentFiles(files) ? files : {}, versionUrl, manifest };
};

export interface LoadOptions {
  /** Content base URL; empty = the bundled configuration (`bundled`) */
  baseUrl?: string;
  /** The configuration bundled with the app (its content build), `null` when the build has none */
  bundled?: () => Promise<unknown>;
  /** For the requests to the base URL (a token, a proxy); default `fetch` */
  fetch?: typeof fetch;
  /**
   * Where the configuration's `/assets/content/…` paths resolve when it is not the version folder itself – the
   * editor's draft points at the files of the version it was imported from
   */
  assetsBase?: string;
  texts?: LoadTexts;
  /** The editor's stage: nodes may use component types its Code tab's plugins register at Run (`GuardOptions`) */
  unknownComponentTypes?: GuardOptions["unknownComponentTypes"];
  /** What this app knows of the contract (default `currentFeatures()`; a test reads content as an older app would) */
  knownFeatures?: KnownFeatures;
  /**
   * The editor: targets switched off (`enabled: false`, #249) stay, with their entries, so they can be worked on. The
   * reader (default) leaves them and their entries out – they are not in its experience.
   */
  keepSwitchedOffTargets?: boolean;
}

/** The content's version when it names a content schema – checked before the guard reads a shape it may not know */
const versionOf = (raw: unknown): ConfigurationVersion | null => {
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return null;
  const version = (raw as { version?: unknown }).version;
  if (typeof version !== 'object' || version === null) return null;
  const { version: app, schema } = version as Record<string, unknown>;
  return parseContentSchema(schema) ? { version: String(app ?? ''), timestamp: '', schema: schema as string } : null;
};

/**
 * Loads the game configuration – once, before the app starts. Resolves with the startup error (also kept
 * for `getConfigurationError()`) or null when the app can start.
 */
export const loadGameConfiguration = async (options: LoadOptions = {}): Promise<ErrorInfo | null> => {
  const baseUrl = (options.baseUrl ?? '').replace(/\/+$/, '');
  const texts = options.texts ?? DEFAULT_TEXTS;
  const known = options.knownFeatures ?? currentFeatures();
  configurationError = null;
  skippedFeatures = [];
  try {
    let raw: unknown;
    let files: ContentFiles = {};
    let versionUrl = '';
    if (baseUrl) {
      ({ raw, files, versionUrl } = await fetchFromContentBase(baseUrl, options.fetch ?? fetch));
    } else {
      if (!options.bundled) throw new Error('No content base URL and no bundled configuration');
      raw = await options.bundled();
      if (raw === null) throw new Error('This build loads its content from the CDN but has no content base URL');
    }
    // A MAJOR this app cannot read is refused before the guard (its shape may differ); a broken version is the guard's
    const named = versionOf(raw);
    configurationError = named && checkContentCompatibility(named, texts, known.schema);
    if (configurationError) {
      console.error(`[game-config] ${configurationError.details?.[0]}`);
      apply(EMPTY_CONFIGURATION, '', {});
      return configurationError;
    }
    const { config: readable, skipped } = skipUnknownFeatures(raw, known, { unknownComponentTypes: options.unknownComponentTypes });
    assertGameConfiguration(readable, { unknownComponentTypes: options.unknownComponentTypes });
    const { version } = readable;
    console.info(`[game-config] Content ${(version.hash ?? '').slice(0, 12)} (schema ${version.schema}, built with app ${version.version}) from ${versionUrl || 'the app'}.`);
    skippedFeatures = skipped;
    if (skipped.length) {
      console.warn(`[game-config] Content schema ${version.schema} is newer than this app's ${known.schema} – skipped what it does not know:\n  - ${skipped.map(describeSkipped).join('\n  - ')}`);
    }
    apply(readable, (options.assetsBase ?? '').replace(/\/+$/, '') || versionUrl, files, options.keepSwitchedOffTargets ?? false);
  } catch (error) {
    const invalid = error instanceof GameConfigurationError;
    const problems = invalid ? error.problems : [String(error instanceof Error ? error.message : error)];
    console.error(`[game-config] ${invalid ? 'Invalid game configuration' : 'Game configuration not loaded'}:\n  - ${problems.join('\n  - ')}`);
    configurationError = {
      code: invalid ? ErrorCode.GAME_CONFIGURATION_INVALID : ErrorCode.NETWORK_ERROR,
      msg: texts.contentNotLoaded,
      type: 'critical',
      details: problems,
    };
    skippedFeatures = [];
    apply(EMPTY_CONFIGURATION, '', {});
  }
  return configurationError;
};

/** Set when the game configuration is invalid or could not be loaded; the app must not start then */
export const getConfigurationError = (): ErrorInfo | null => configurationError;

/** What the loaded content has that this app does not know (newer content, RULES #10) – empty when nothing was skipped */
let skippedFeatures: readonly SkippedFeature[] = [];

/** The features of the loaded content this app skipped – the reader's update hint, the editor's warning */
export const getSkippedFeatures = (): readonly SkippedFeature[] => skippedFeatures;

let config: GameConfiguration = EMPTY_CONFIGURATION;
/** URL of the loaded content version on the CDN (`<base>/<build>`; '' when the content is served with the app) */
let contentBase = '';
/** files.json of the loaded content (empty for content served with the app) */
let contentFiles: ContentFiles = {};
let entries: Entry[] = [];
let scenes: Scene[] = [];
let sceneById = new Map<string, Scene>();
let entryById = new Map<string, Entry>();
let targetById = new Map<string, Target>();
/** The rules' variables: declared or inferred from their first set (schema 5.0, B7) */
let variables: ReadonlyMap<string, VariableInfo> = new Map();

/** URL of a content file given its path in the configuration (`/assets/content/…`) */
export const contentUrl = (path: string): string => (path.startsWith(CONTENT_PATH) && contentBase ? `${contentBase}${path}` : path);

/** A URL of the loaded content (same origin or the content CDN) – as opposed to external links */
export const isContentUrl = (src?: string): src is string =>
  !!src && (src.startsWith(CONTENT_PATH) || (contentBase !== '' && src.startsWith(`${contentBase}${CONTENT_PATH}`)));

/** The path in the configuration (`/assets/content/…`) of a content URL, or null for other URLs */
export const contentPathOf = (src: string): string | null => {
  if (!isContentUrl(src)) return null;
  const path = src.startsWith(CONTENT_PATH) ? src : src.slice(contentBase.length);
  return path.split(/[?#]/)[0];
};

/** Where the content files come from: the loaded version's folder on the CDN, '' when served with the app */
export const getContentBaseUrl = (): string => contentBase;

/** Sizes in bytes by content URL from files.json (CDN content), null for content served with the app */
export const getContentSizes = (): Record<string, number> | null =>
  contentBase ? Object.fromEntries(Object.entries(contentFiles).map(([path, file]) => [contentUrl(path), file.size])) : null;

const withContentUrls = (raw: GameConfiguration, base: string): GameConfiguration => {
  if (!base) return raw;
  const url = (path?: string) => (path === undefined ? undefined : contentUrl(path));
  const levelUrls = (levels: readonly MediaLevelData[]): MediaLevelData[] => levels.map(level => ({ ...level, src: contentUrl(level.src) }));
  const node = (e: EntityData): EntityData => ({
    ...e,
    assets: e.assets.map(a => ({
      ...a,
      src: contentUrl(a.src),
      ...(a.textures ? { textures: a.textures.map(set => ({ ...set, sizes: set.sizes.map(size => ({ ...size, src: contentUrl(size.src) })) })) } : {}),
      ...(a.levels ? { levels: levelUrls(a.levels) } : {}),
    })),
    ...(e.children ? { children: e.children.map(node) } : {}),
  });
  const entity = (e?: EntityData | EntityRefData): EntityData | EntityRefData | undefined =>
    e && !isEntityRef(e) ? node(e) : e;
  return {
    ...raw,
    entities: Object.fromEntries(Object.entries(raw.entities).map(([id, e]) => [id, entity(e) as EntityData])),
    entries: raw.entries.map(e => ({
      ...e,
      ...(e.image !== undefined ? { image: url(e.image) } : {}),
      ...(e.imageLevels ? { imageLevels: levelUrls(e.imageLevels) } : {}),
      ...(e.target ? {
        target: {
          ...e.target,
          imageSrc: contentUrl(e.target.imageSrc),
          ...(e.target.recognition ? { recognition: { ...e.target.recognition, imageSrc: contentUrl(e.target.recognition.imageSrc) } } : {}),
          entity: entity(e.target.entity),
        },
      } : {}),
    })),
  };
};

const apply = (raw: GameConfiguration, base: string, files: ContentFiles, keepSwitchedOff = false): void => {
  contentBase = base;
  contentFiles = files;
  config = withContentUrls(raw, base);
  variables = variableTypes(config.experience?.variables, contentRuleLists(config));

  const resolveEntity = (entity?: EntityData | EntityRefData): EntityData | undefined =>
    entity && isEntityRef(entity) ? config.entities[entity.ref] : entity;


  // A switched-off target and its entry are not in the reader's experience: no image target, no list entry, not counted
  const inExperience = keepSwitchedOff ? config.entries : config.entries.filter(isEntryInExperience);
  entries = inExperience.map(entry => {
    const { target, ...data } = entry;
    const sceneId = entry.scene;
    return {
      ...data,
      sceneId,
      ...(target
        ? { target: { ...target, entity: resolveEntity(target.entity), entryId: data.id, sceneId } }
        : {}),
    };
  });

  scenes = config.scenes.map(data => {
    const sceneEntries = entries.filter(e => e.sceneId === data.id);
    return {
      ...data,
      entries: sceneEntries,
      targets: sceneEntries.flatMap(e => (e.target ? [e.target] : [])),
    };
  });

  sceneById = new Map(scenes.map(s => [s.id, s]));
  entryById = new Map(entries.map(e => [e.id, e]));
  targetById = new Map(scenes.flatMap(s => s.targets.map(t => [t.id, t] as const)));
};

/** Version of the game configuration (content build): version, timestamp, hash, schema */
export const getConfigVersion = (): ConfigurationVersion => config.version;

export const getExperience = (): ExperienceData => config.experience;

/** The rules' typed variables (schema 5.0, B7): the experience's declarations, else each one's first set */
export const getVariableTypes = (): ReadonlyMap<string, VariableInfo> => variables;

/**
 * The passes a scene shows: its own list (an override – `[]` turns the experience's off there), else the experience's (the master
 * for every scene), else none. Pure, for the tests.
 */
export const passesFor = (scene: { passes?: PassData[] } | undefined, experience: { passes?: PassData[] }): PassData[] =>
  scene?.passes ?? experience.passes ?? [];

/** The post-processing passes over a scene's rendered image: the scene's own list, else the experience's, else none */
export const getPasses = (sceneId: string): PassData[] => passesFor(sceneById.get(sceneId), config.experience);


export const getInitialSceneId = (): string => config.initialScene;

export const getScenes = (): Scene[] => [...scenes];

export const getScene = (id: string): Scene | undefined => sceneById.get(id);

/**
 * The scenes next to `sceneId` in experience order (the ones the scene menu reaches next) – what is preloaded
 * ahead of a switch (target images + content, and the tracker's image targets)
 */
export const getNeighbourScenes = (sceneId: string, range = 1): Scene[] => {
  const index = scenes.findIndex(s => s.id === sceneId);
  return index === -1 ? [] : scenes.filter((_, i) => i !== index && Math.abs(i - index) <= range);
};

export { entryPlaceLabel, sceneLabel, surfaceOf } from './scenes';

/** The groups scenes name – a label over several scenes */
export const getGroups = (): GroupData[] => [...(config.groups ?? [])];

/** The place an entry is on as the reader finds it: its surface's label, else its scene's (`./scenes.ts`) */
export const getEntryPlaceLabel = (entry: Pick<Entry, 'surface' | 'sceneId'>): string =>
  entryPlaceLabel(entry, sceneById.get(entry.sceneId));

export const getEntries = (): Entry[] => [...entries];

export const getEntry = (id: string): Entry | undefined => entryById.get(id);

/** Targets of a scene in entry order */
export const getTargets = (sceneId: string): Target[] => sceneById.get(sceneId)?.targets ?? [];

export const getTarget = (id: string): Target | undefined => targetById.get(id);

/** The assets of an entity's whole node tree, root first */
export const entityAssets = (entity: EntityData | undefined): AssetData[] =>
  entity ? [...entity.assets, ...(entity.children ?? []).flatMap(entityAssets)] : [];

/** Every node of an entity's tree with its path from the root (`[]` = the root, `[1, 0]` = the second child's first) */
export const entityNodes = (entity: EntityData, path: number[] = []): Array<{ node: EntityData; path: number[] }> =>
  [{ node: entity, path }, ...(entity.children ?? []).flatMap((child, i) => entityNodes(child, [...path, i]))];

/** Assets of all entities (optionally of one scene), deduplicated by id */
export const getAssets = (sceneId?: string): AssetData[] => {
  const targets = sceneId ? getTargets(sceneId) : [...targetById.values()];
  const assets = new Map<string, AssetData>();
  targets.forEach(t => entityAssets(t.entity).forEach(a => assets.set(a.id, a)));
  return [...assets.values()];
};

/** The plugins the content declares, their modules as URLs of the loaded version */
export const getPlugins = (): PluginData[] => (config.plugins ?? []).map(plugin => ({ ...plugin, src: contentUrl(plugin.src) }));

/**
 * The subtitle file of a video for a language (the video as its asset names it, a URL of the loaded version): the
 * language's track or its fallback (`pickSubtitleTrack`); undefined for a video without subtitles
 */
export const getSubtitleTrack = (media: string, language: string): string | undefined => {
  const subtitle = config.subtitles?.find(s => contentUrl(s.media) === media);
  const track = subtitle && pickSubtitleTrack(subtitle.tracks, language);
  return track === undefined ? undefined : contentUrl(track);
};

/** Every subtitle file of the experience's videos (optionally of one scene's), as URLs of the loaded version – for preloading */
export const getSubtitleFiles = (sceneId?: string): string[] => {
  const media = sceneId === undefined ? null : new Set(getAssets(sceneId).map(asset => asset.src));
  return (config.subtitles ?? [])
    .filter(s => !media || media.has(contentUrl(s.media)))
    .flatMap(s => Object.values(s.tracks).map(contentUrl));
};

/** A scene's credit lines (#292): the scene and the lines of the files its entries use, in entry order, each once */
export interface SceneCredits {
  scene: Scene;
  lines: string[];
}

/**
 * The experience's credit lines per scene (the configuration's `credits`, schema 2.33): a scene lists the line of every file
 * its entries in the experience name anywhere (in entry order) – the target image, the entry's picture, the entity's files (a shared entity's
 * too). Scenes without a line are left out; a line a scene repeats is listed once.
 */
export const getCredits = (): SceneCredits[] => {
  const credited = Object.entries(config.credits ?? {}).map(([file, line]) => [contentUrl(file), line] as const);
  if (!credited.length) return [];
  return scenes.flatMap(scene => {
    const lines: string[] = [];
    for (const entry of scene.entries) {
      const text = JSON.stringify(entry);
      for (const [url, line] of credited) {
        if (!lines.includes(line) && text.includes(JSON.stringify(url))) lines.push(line);
      }
    }
    return lines.length ? [{ scene, lines }] : [];
  });
};

/** The shared entity (`content/entities/<id>`) a target refers to with `ref`, or undefined for an inline entity */
export const getEntityRef = (targetId: string): string | undefined => {
  const entity = config.entries.find(e => e.target?.id === targetId)?.target?.entity;
  return entity && isEntityRef(entity) ? entity.ref : undefined;
};

/** Ids of the targets whose entity is the shared entity `ref` */
export const getTargetsUsingEntity = (ref: string): string[] =>
  config.entries.flatMap(e => {
    const entity = e.target?.entity;
    return e.target && entity && isEntityRef(entity) && entity.ref === ref ? [e.target.id] : [];
  });
