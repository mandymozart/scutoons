/**
 * Runtime type guards for the game configuration contract (`shared/types/game-config.ts`).
 *
 * JSON imports only give `string` for literal unions and nothing is checked at runtime, so the
 * build runs `assertGameConfiguration` before writing and the app runs it on load.
 */
import {
  ASSET_TYPES,
  type AssetType,
  CONTENT_SCHEMA_VERSION,
  ENTITY_TYPES,
  type EntityRefData,
  type EntityType,
  type GameConfiguration,
  parseContentSchema,
  SURFACE_PLACE_KEYS,
} from "../types/game-config";
import { ENTRY_CATEGORIES, type EntryCategory } from "../types/entry";
import { componentProblems } from "../types/components";
import { materialProblems } from "../types/materials";
import { type PassCheckOptions, passProblems } from "../types/passes";
import { type AppearanceCheckOptions, appearanceProblems } from "../types/appearance";
import { environmentProblems } from "../types/environment";
import { pluginProblems, recordedRegistrations } from "../types/plugins";
import { NODE_TYPE_PATTERN, nodeParamProblems, nodeTypeSpec } from "../types/node-types";
import { filterProblems } from "../types/filters";
import { lightProblems } from "../types/lights";
import { textureSetProblems } from "../types/textures";
import { mediaLevelProblems } from "../types/media-levels";
import { placementProblems } from "../types/placement";
import { subtitleProblems } from "../types/subtitles";
import { isObject } from "./primitives";
import { BEHAVIOUR_TYPE, type BehaviourRule, readRules } from "../types/behaviour";
import { logicProblems, logicRules } from "../types/logic";
import { parseAddress } from "../types/address";
import { type RuleList, ruleTypeProblems, variablesProblems, variableTypes } from "../types/variables";

export { isObject } from "./primitives";

/** A pass parameter a newer MINOR adds is ignored here (the newer app reads it); the build and the API refuse it */
const GUARD_PASSES: PassCheckOptions = { unknownParams: "ignore" };

const oneOf = <T extends string>(values: readonly T[]) =>
  (value: unknown): value is T => typeof value === "string" && (values as readonly string[]).includes(value);

export const isEntryCategory: (value: unknown) => value is EntryCategory = oneOf(ENTRY_CATEGORIES);
export const isEntityType: (value: unknown) => value is EntityType = oneOf(ENTITY_TYPES);

/** How deep an entity's node tree may nest (the root is level 0) */
export const MAX_NODE_DEPTH = 8;
export const isAssetType: (value: unknown) => value is AssetType = oneOf(ASSET_TYPES);

export const isEntityRef = (value: unknown): value is EntityRefData =>
  isObject(value) && typeof value.ref === "string";

export class GameConfigurationError extends Error {
  constructor(public readonly problems: string[]) {
    super(`Invalid game configuration (${problems.length} problem(s)):\n  - ${problems.join("\n  - ")}`);
    this.name = "GameConfigurationError";
  }
}

export interface GuardOptions {
  /**
   * Component types the registry does not know (the default, `"declared"`): refused without plugins; with plugins that
   * all recorded what they register (schema 2.14) checked against those specs and refused when none registers them;
   * with a plugin that recorded nothing (an older build) allowed – it registers them at load. Or always allowed
   * (`"allow"`: the editor's stage, where the Code tab's plugins register them at Run). The same for a plugin's node
   * type: checked against the recorded specs, refused when no declared plugin registers it, allowed where unknown – and
   * for a plugin's pass in `passes` (schema 2.21).
   */
  unknownComponentTypes?: "declared" | "allow";
}

/**
 * Throws a `GameConfigurationError` listing every problem (with its path) unless `raw` matches
 * the `GameConfiguration` contract. Also checks references: entity refs exist, ids are unique.
 */
export function assertGameConfiguration(raw: unknown, options: GuardOptions = {}): asserts raw is GameConfiguration {
  const problems: string[] = [];
  const fail = (path: string, message: string) => problems.push(`${path}: ${message}`);

  const str = (obj: Record<string, unknown>, key: string, path: string, optional = false) => {
    const value = obj[key];
    if (value === undefined && optional) return;
    if (typeof value !== "string" || (!optional && value === "")) fail(`${path}.${key}`, "expected a non-empty string");
  };
  const arr = (obj: Record<string, unknown>, key: string, path: string): unknown[] => {
    const value = obj[key];
    if (Array.isArray(value)) return value;
    fail(`${path}.${key}`, "expected an array");
    return [];
  };
  const obj = (value: unknown, path: string): Record<string, unknown> | null => {
    if (isObject(value)) return value;
    fail(path, "expected an object");
    return null;
  };

  // The declared plugins' recorded specs (schema 2.14): what they register is known without running them. A plugin that
  // recorded nothing registers what it registers at load – a type the registry does not know may be its own
  const root0 = isObject(raw) ? raw : {};
  const declared = Array.isArray(root0.plugins) ? root0.plugins : [];
  const recorded = recordedRegistrations(declared);
  const allowUnknown = options.unknownComponentTypes === "allow" || (declared.length > 0 && !recorded.complete);
  // Content of a newer schema may have built-in node types this app does not know: those only warn (below)
  const ours = parseContentSchema(CONTENT_SCHEMA_VERSION)!;
  const theirs = isObject(root0.version) ? parseContentSchema(root0.version.schema) : null;
  const newerContent = !!theirs && (theirs.major > ours.major || (theirs.major === ours.major && theirs.minor > ours.minor));
  const refuseUnknownNodes = declared.length > 0 && recorded.complete && !newerContent;
  // A plugin's pass (schema 2.21): checked against the recorded spec like a built-in pass; a type no declared plugin
  // recorded is refused (as a 2.20 guard refuses every plugin pass), unless a plugin is unknown until it runs
  const passOptions: PassCheckOptions = {
    ...GUARD_PASSES,
    spec: type => recorded.passes.get(type),
    pluginTypes: [...recorded.passes.keys()],
    unknownTypes: allowUnknown ? "allow" : "refuse",
  };

  // A plugin's look (schema 2.29): as a plugin's pass. A moment or a parameter a newer MINOR adds is left to the newer app
  const appearanceOptions: AppearanceCheckOptions = {
    unknown: "ignore",
    spec: type => recorded.appearances.get(type),
    pluginTypes: [...recorded.appearances.keys()],
    unknownTypes: allowUnknown ? "allow" : "refuse",
  };

  // Schema 5.0 (#334): where rules go (`goto`) and the nodes a scene's or the experience's rules name – checked once the
  // scenes and targets are known (below)
  const gotos: Array<{ address: string; path: string }> = [];
  const nodeAddresses: Array<{ address: string; path: string; scene: string | null }> = [];
  /** Every list of rules in content order – the variables' types are inferred and checked over them (B7) */
  const ruleLists: RuleList[] = [];
  const collect = (rules: BehaviourRule[], path: string, scene: string | null, named: boolean) =>
    ruleLists.push({ rules, path }) && rules.forEach((rule, i) => rule.do.forEach((action, j) => {
      const at = `${path}[${i}].do[${j}]`;
      if (action.action === "goto") gotos.push({ address: action.address, path: `${at}.goto` });
      else if (named && action.action !== "set" && action.action !== "emit" && action.node) nodeAddresses.push({ address: action.node, path: at, scene });
    }));

  const entity = (value: unknown, path: string, depth = 0) => {
    const e = obj(value, path);
    if (!e) return;
    if (Array.isArray(e.components)) e.components.forEach((component, i) => {
      if (isObject(component) && component.type === BEHAVIOUR_TYPE) collect(readRules(component.rules), `${path}.components[${i}].rules`, null, false);
    });
    if (e.id !== undefined && typeof e.id !== "string") fail(`${path}.id`, "expected a string");
    if (e.name !== undefined && typeof e.name !== "string") fail(`${path}.name`, "expected a string");
    if (e.text !== undefined && typeof e.text !== "string") fail(`${path}.text`, "expected a string");
    if (e.color !== undefined && !(typeof e.color === "string" && /^#[0-9a-f]{6}$/i.test(e.color))) fail(`${path}.color`, "expected #rrggbb");
    if (e.children !== undefined) {
      if (!Array.isArray(e.children)) fail(`${path}.children`, "expected an array");
      else if (depth >= MAX_NODE_DEPTH) fail(`${path}.children`, `nested deeper than ${MAX_NODE_DEPTH} levels`);
      else e.children.forEach((child, i) => entity(child, `${path}.children[${i}]`, depth + 1));
    }
    // A plugin's node type (schema 2.13): registered or recorded by a declared plugin (2.14) – its parameters are checked
    // against its spec; declared plugins recorded what they register and none registers it – refused (unless the content
    // is of a newer schema, below); a declared plugin recorded nothing – it may register it when it loads. Any other
    // type this contract does not know is a later MINOR's: the engine renders nothing for it and keeps its children
    // (the content build and the editor's API refuse it – an author's typo never gets this far)
    if (typeof e.type !== "string" || e.type === "") fail(`${path}.type`, `expected a node type (${ENTITY_TYPES.join(", ")})`);
    else if (!isEntityType(e.type)) {
      const spec = nodeTypeSpec(e.type) ?? recorded.nodeTypes.get(e.type);
      if (spec) nodeParamProblems(spec, e.params, `${path}.params`, e.type).forEach(problem => problems.push(problem));
      else if (allowUnknown && NODE_TYPE_PATTERN.test(e.type)) {
        // the plugin registers it when it loads
      } else if (refuseUnknownNodes) {
        const plugins = [...recorded.nodeTypes.keys()];
        fail(`${path}.type`, `"${e.type}" is not a node type – neither built in (${ENTITY_TYPES.join(", ")}) nor registered by a plugin of the experience${plugins.length ? ` (${plugins.join(", ")})` : ""}`);
      } else {
        console.warn(`[game-config] ${path}.type: "${e.type}" is not a node type this app knows (${ENTITY_TYPES.join(", ")}) – it renders nothing, its children stay`);
      }
    }
    if (e.light !== undefined) {
      if (e.type !== "light") fail(`${path}.light`, "only light nodes have a light");
      lightProblems(e.light, `${path}.light`).forEach(problem => problems.push(problem));
    }
    arr(e, "assets", path).forEach((a, i) => {
      const asset = obj(a, `${path}.assets[${i}]`);
      if (!asset) return;
      str(asset, "id", `${path}.assets[${i}]`);
      str(asset, "src", `${path}.assets[${i}]`);
      if (!isAssetType(asset.assetType)) {
        fail(`${path}.assets[${i}].assetType`, `"${asset.assetType}" is not one of ${ASSET_TYPES.join(", ")}`);
      }
      if (asset.textures !== undefined) {
        textureSetProblems(asset.textures, asset.assetType).forEach(problem => fail(`${path}.assets[${i}].textures`, problem));
      }
      if (asset.levels !== undefined) {
        mediaLevelProblems(asset.levels, asset.assetType).forEach(problem => fail(`${path}.assets[${i}].levels`, problem));
      }
    });
    if (e.params !== undefined && !isObject(e.params)) fail(`${path}.params`, "expected an object");
    placementProblems(e.params, `${path}.params`).forEach(problem => problems.push(problem));
    if (e.filters !== undefined) {
      if (e.type !== "video") fail(`${path}.filters`, "filters only work on video entities");
      filterProblems(e.filters, `${path}.filters`).forEach(problem => problems.push(problem));
    }
    if (e.components !== undefined) {
      componentProblems(e.components, `${path}.components`, typeof e.type === "string" ? e.type : undefined, {
        unknownTypes: allowUnknown ? "allow" : "refuse", specs: recorded.components, appearance: appearanceOptions,
      }).forEach(problem => problems.push(problem));
    }
    // A plugin's material (schema 2.17): checked against the registry or the declared plugins' recorded specs, like a component
    if (e.material !== undefined) {
      materialProblems(e.material, `${path}.material`, typeof e.type === "string" ? e.type : undefined, { unknownTypes: allowUnknown ? "allow" : "refuse", specs: recorded.materials })
        .forEach(problem => problems.push(problem));
    }
  };

  const root = obj(raw, "config");
  if (!root) throw new GameConfigurationError(problems);

  const version = obj(root.version, "version");
  if (version) {
    str(version, "version", "version");
    str(version, "timestamp", "version");
    str(version, "hash", "version", true);
    if (!parseContentSchema(version.schema)) fail("version.schema", 'expected "MAJOR.MINOR", e.g. "1.0"');
  }

  const experience = obj(root.experience, "experience");
  if (experience) {
    ["id", "title"].forEach(key => str(experience, key, "experience"));
    // The print data lives in `book` (schema 4.0) – on the top level it is an error; the source of truth is gone
    ["author", "publisher"].forEach(key => {
      if (experience[key] !== undefined) fail(`experience.${key}`, "belongs to the printed book (experience.book)");
    });
    if (experience.sourceOfTruth !== undefined) fail("experience.sourceOfTruth", "is gone – every experience adds and removes its scenes");
    if (experience.book !== undefined) {
      const book = obj(experience.book, "experience.book");
      if (book) {
        str(book, "author", "experience.book");
        str(book, "publisher", "experience.book", true);
        if (book.sourceOfTruth !== undefined) fail("experience.book.sourceOfTruth", "is gone – every experience adds and removes its scenes");
      }
    }
    // Schema 4.2 (#306): any text – one this app does not know reads as "original" (the engine's `recognitionModeOf`)
    if (experience.recognition !== undefined && typeof experience.recognition !== "string") fail("experience.recognition", `expected "original" or "prepared"`);
    if (experience.passes !== undefined) passProblems(experience.passes, "experience.passes", passOptions).forEach(problem => problems.push(problem));
    if (experience.environment !== undefined) environmentProblems(experience.environment, "experience.environment", false).forEach(problem => problems.push(problem));
    if (experience.appearance !== undefined) appearanceProblems(experience.appearance, "experience.appearance", appearanceOptions).forEach(problem => problems.push(problem));
    if (experience.variables !== undefined) variablesProblems(experience.variables, "experience.variables").forEach(problem => problems.push(problem));
    if (experience.logic !== undefined) {
      logicProblems(experience.logic, "experience.logic", "experience").forEach(problem => problems.push(problem));
      collect(logicRules(experience.logic), "experience.logic.rules", null, true);
    }
  }

  str(root, "initialScene", "config");
  if (root.plugins !== undefined) pluginProblems(root.plugins).forEach(problem => problems.push(problem));
  if (root.subtitles !== undefined) subtitleProblems(root.subtitles, "subtitles").forEach(problem => problems.push(problem));
  if (root.credits !== undefined) {
    const credits = root.credits;
    if (typeof credits !== "object" || credits === null || Array.isArray(credits)) fail("credits", "expected an object of file → credit line");
    else Object.entries(credits).forEach(([file, line]) => {
      if (typeof line !== "string" || !line.trim()) fail(`credits[${JSON.stringify(file)}]`, "expected a credit line (text)");
    });
  }

  // Groups: a label over several scenes, named by a scene's `group`
  const groupIds = new Set<string>();
  if (root.groups !== undefined) arr(root, "groups", "config").forEach((g, i) => {
    const path = `groups[${i}]`;
    const group = obj(g, path);
    if (!group) return;
    ["id", "label"].forEach(key => str(group, key, path));
    if (typeof group.id === "string") {
      if (groupIds.has(group.id)) fail(`${path}.id`, `duplicate group id "${group.id}"`);
      groupIds.add(group.id);
    }
  });

  const sceneIds = new Set<string>();
  /** The surface ids of each scene – an entry's `surface` and a target's `place.surface` must be one of its scene's */
  const surfacesOf = new Map<string, Set<string>>();
  arr(root, "scenes", "config").forEach((s, i) => {
    const path = `scenes[${i}]`;
    const scene = obj(s, path);
    if (!scene) return;
    ["id", "label"].forEach(key => str(scene, key, path));
    if (scene.passes !== undefined) passProblems(scene.passes, `${path}.passes`, passOptions).forEach(problem => problems.push(problem));
    if (scene.logic !== undefined) {
      logicProblems(scene.logic, `${path}.logic`, "scene").forEach(problem => problems.push(problem));
      collect(logicRules(scene.logic), `${path}.logic.rules`, typeof scene.id === "string" ? scene.id : null, true);
    }
    const surfaceIds = new Set<string>();
    const surfaces = arr(scene, "surfaces", path);
    if (Array.isArray(scene.surfaces) && surfaces.length === 0) fail(`${path}.surfaces`, "a scene has at least one surface");
    surfaces.forEach((f, j) => {
      const fPath = `${path}.surfaces[${j}]`;
      const surface = obj(f, fPath);
      if (!surface) return;
      str(surface, "id", fPath);
      (["widthMm", "heightMm"] as const).forEach(key => {
        const size = surface[key];
        if (typeof size !== "number" || !Number.isFinite(size) || size <= 0) fail(`${fPath}.${key}`, "expected a size in mm (> 0)");
      });
      str(surface, "label", fPath, true);
      // Where it lies in the scene: six finite numbers
      const place = obj(surface.place, `${fPath}.place`);
      if (place) SURFACE_PLACE_KEYS.forEach(key => {
        if (typeof place[key] !== "number" || !Number.isFinite(place[key])) fail(`${fPath}.place.${key}`, "expected a number");
      });
      if (typeof surface.id === "string") {
        if (surfaceIds.has(surface.id)) fail(`${fPath}.id`, `duplicate surface id "${surface.id}"`);
        surfaceIds.add(surface.id);
      }
    });
    if (scene.group !== undefined) {
      if (typeof scene.group !== "string" || scene.group === "") fail(`${path}.group`, "expected a group id");
      else if (!groupIds.has(scene.group)) fail(`${path}.group`, `unknown group "${scene.group}"`);
    }
    if (typeof scene.id === "string") {
      surfacesOf.set(scene.id, surfaceIds);
      if (sceneIds.has(scene.id)) fail(`${path}.id`, `duplicate scene id "${scene.id}"`);
      sceneIds.add(scene.id);
    }
  });
  if (typeof root.initialScene === "string" && !sceneIds.has(root.initialScene)) {
    fail("config.initialScene", `unknown scene "${root.initialScene}"`);
  }

  const entities = obj(root.entities, "entities") ?? {};
  Object.entries(entities).forEach(([id, e]) => entity(e, `entities.${id}`));

  /** A surface of the scene, by id (`what` names the field in the message) */
  const surfaceProblem = (sceneId: unknown, surfaceId: unknown, path: string, what: string) => {
    if (typeof surfaceId !== "string" || surfaceId === "") return fail(path, "expected a surface id");
    if (typeof sceneId === "string" && sceneIds.has(sceneId) && !surfacesOf.get(sceneId)?.has(surfaceId)) {
      fail(path, `scene "${sceneId}" has no surface "${surfaceId}" (${what})`);
    }
  };

  const entryIds = new Set<string>();
  const targetIds = new Set<string>();
  /** Each target's scene (its entry's) – a scene's rules name their own scene's targets */
  const targetScenes = new Map<string, string>();
  arr(root, "entries", "config").forEach((e, i) => {
    const path = `entries[${i}]`;
    const entry = obj(e, path);
    if (!entry) return;
    ["id", "title"].forEach(key => str(entry, key, path));
    ["author", "image", "caption", "media"].forEach(key => str(entry, key, path, true));
    if (entry.imageLevels !== undefined) {
      if (entry.image === undefined) fail(`${path}.imageLevels`, "levels of a picture the entry does not have (no image)");
      mediaLevelProblems(entry.imageLevels, "image").forEach(problem => fail(`${path}.imageLevels`, problem));
    }
    if (typeof entry.body !== "string") fail(`${path}.body`, "expected a string");
    if (typeof entry.scene !== "string" || entry.scene === "") fail(`${path}.scene`, "expected a scene id");
    else if (!sceneIds.has(entry.scene)) fail(`${path}.scene`, `unknown scene "${entry.scene}"`);
    if (entry.surface !== undefined) surfaceProblem(entry.scene, entry.surface, `${path}.surface`, "the entry's surface");
    if (!isEntryCategory(entry.category)) {
      fail(`${path}.category`, `"${entry.category}" is not one of ${ENTRY_CATEGORIES.join(", ")}`);
    }
    if (!arr(entry, "tags", path).every(tag => typeof tag === "string")) fail(`${path}.tags`, "expected strings");
    if (typeof entry.id === "string") {
      if (entryIds.has(entry.id)) fail(`${path}.id`, `duplicate entry id "${entry.id}"`);
      entryIds.add(entry.id);
    }

    if (entry.target === undefined) return;
    const tPath = `${path}.target`;
    const target = obj(entry.target, tPath);
    if (!target) return;
    str(target, "id", tPath);
    str(target, "imageSrc", tPath);
    if (target.place !== undefined) {
      const place = obj(target.place, `${tPath}.place`);
      if (place) {
        (["x", "y", "width"] as const).forEach(key => {
          if (typeof place[key] !== "number" || !Number.isFinite(place[key])) fail(`${tPath}.place.${key}`, "expected a number (mm)");
        });
        if (place.height !== undefined && (typeof place.height !== "number" || !Number.isFinite(place.height))) fail(`${tPath}.place.height`, "expected a number (mm)");
        // The surface it lies on: required when its scene has several
        if (place.surface !== undefined) surfaceProblem(entry.scene, place.surface, `${tPath}.place.surface`, "the target's place");
        else if (typeof entry.scene === "string" && (surfacesOf.get(entry.scene)?.size ?? 0) > 1) {
          fail(`${tPath}.place.surface`, `scene "${entry.scene}" has several surfaces – name the one the target lies on`);
        }
      }
    }
    if (target.enabled !== undefined && typeof target.enabled !== "boolean") fail(`${tPath}.enabled`, "expected true or false");
    if (target.recognitionOff !== undefined && typeof target.recognitionOff !== "boolean") fail(`${tPath}.recognitionOff`, "expected true or false");
    if (target.recognition !== undefined) {
      const recognition = obj(target.recognition, `${tPath}.recognition`);
      if (recognition) {
        str(recognition, "imageSrc", `${tPath}.recognition`);
        (["width", "height"] as const).forEach(key => {
          const value = recognition[key];
          if (typeof value !== "number" || !Number.isInteger(value) || value < 1) fail(`${tPath}.recognition.${key}`, "expected a whole number of px");
        });
      }
    }
    if (typeof target.id === "string") {
      if (targetIds.has(target.id)) fail(`${tPath}.id`, `duplicate target id "${target.id}"`);
      targetIds.add(target.id);
      if (typeof entry.scene === "string") targetScenes.set(target.id, entry.scene);
    }
    if (target.entity === undefined) return;
    if (isEntityRef(target.entity)) {
      if (!(target.entity.ref in entities)) fail(`${tPath}.entity.ref`, `unknown entity "${target.entity.ref}"`);
    } else {
      entity(target.entity, `${tPath}.entity`);
    }
  });

  // A goto names a scene of the experience; a scene's or the experience's rule a target of it – a scene's rule one of its
  // own scene (only the open scene's entities are built). The shapes were checked above; a node's name is the engine's to find
  gotos.forEach(({ address, path }) => {
    const to = parseAddress(address);
    if (to?.scheme === "scene" && !sceneIds.has(to.id)) fail(path, `unknown scene "${to.id}"`);
  });
  // Typed variables (schema 5.0, B7): declared or inferred from the first set; a set, a test or a condition of another type
  const declarations = isObject(root.experience) ? root.experience.variables : undefined;
  ruleTypeProblems(ruleLists, variableTypes(declarations, ruleLists)).forEach(problem => problems.push(problem));
  nodeAddresses.forEach(({ address, path, scene }) => {
    const at = parseAddress(address);
    if (at?.scheme !== "target") return;
    const own = targetScenes.get(at.id);
    if (!targetIds.has(at.id)) fail(path, `unknown target "${at.id}"`);
    else if (scene !== null && own !== scene) fail(path, `target "${at.id}" lies in scene "${own}" – a scene's rules act on its own targets`);
  });

  if (problems.length > 0) throw new GameConfigurationError(problems);
}
