import * as THREE from "three";
import type { Object3D } from "three";
import {
  type ComponentData,
  type ComponentSpec,
  type AppearanceSpec,
  componentSpecProblems,
  isBuiltInAppearanceType,
  type EntityData,
  type MaterialSpec,
  materialSpecProblems,
  type NodeTypeSpec,
  nodeTypeSpecProblems,
  PLUGIN_API_VERSION,
  pluginApiCompatible,
  type PluginData,
  type PassSpec,
  type ActionsOf,
  type EventsOf,
  numberProblem,
  PARAM_BUILDERS,
  type ParamBuilders,
  type ParamsOf,
  registerComponentSpec,
  registerMaterialSpec,
  registerNodeTypeSpec,
  registerTextureSpec,
  resolveNodeParams,
  resolveParams,
  type Target,
  type TextureSpec,
  textureSpecProblems,
  unregisterComponentSpec,
  unregisterMaterialSpec,
  unregisterNodeTypeSpec,
  unregisterTextureSpec,
} from "../types/index";
import { contentUrl } from "../content/game-config";
import { type LoadedAsset, loadGltf } from "./assets";
import { dropPluginListeners, nextRun, type PluginEvents, type PluginProgress, type PluginScene, type PluginVars, runtimeApi } from "./plugin-runtime";
import { type PassFactory, passRegistration, registeredPassTypes, registerPass, unregisterPassesOf } from "./post/registry";
import { deleteMaterialRegistration, type MaterialBuild, materialRegistration, setMaterialRegistration } from "./materials";
import { deleteTextureRegistration, resolveTextures, sameTextures, setTextureRegistration, type TextureDraw, textureRegistration } from "./textures";
import {
  type AppearanceFactory, appearanceRegistration, registerAppearance, registeredAppearanceTypes, unregisterAppearancesOf,
} from "./appearances";
import { collectFeedbacks, createFeedback, disposePluginFeedbacks, type Feedback, type FeedbackOptions, type OwnedFeedbacks, shadertoyFragment } from "./feedback";

/**
 * The plugin API (version `PLUGIN_API_VERSION`, docs/plugins.md): an experience's own code
 * extends the engine without touching it. A plugin module exports `register(api)` and calls the registries –
 *
 *   api.registerComponent("spin", SPEC, ({ object, params }) => ({ update: delta => { object.rotation.y += … } }))
 *   api.registerLayer("tint", SPEC, { glsl: "return mix(rgba, vec4(color, rgba.a), opacity);" })
 *   api.registerPass("halo", PASS_SPEC, ({ shaderPass }) => shaderPass(FRAGMENT))
 *   api.registerNodeType("sparkle", NODE_SPEC, ({ params, three }) => ({ object: new three.Points(…) }))
 *   api.registerMaterial("water", MATERIAL_SPEC, ({ params, time, three }) => new three.ShaderMaterial({ uniforms: { time }, … }))
 *   api.registerTexture("caustics", TEXTURE_SPEC, ({ params, three }) => ({ texture: new three.CanvasTexture(canvas), update }))
 *   api.registerAppearance("softFade", LOOK_SPEC, ({ params, materials }) => ({ duration: params.duration, update: visible => … }))
 *
 * – with the specs of `shared/types/components.ts` (a pass: `shared/types/passes.ts`, a node type:
 * `shared/types/node-types.ts`), so the editor's Entity panel, the content build and the guard read a plugin's
 * component like the built-in `animation`. A **component** runs code on a node (`applyComponents`, called by the
 * entity registry when a node is built); a **material layer** is a piece of GLSL the engine composes into the node's
 * material (`utils/layers.ts`); a **pass** is a post-processing effect over the rendered AR image
 * (`post/registry.ts`); a **node type** builds a node's own three.js object (`buildPluginNode`, called by the entity
 * registry for a type without a built-in builder – the node is placed, holds children, components and layers like
 * any); a **material** (`materials.ts`) replaces the material of a node's meshes (`material: { type, params }`);
 * a **procedural texture** (`textures.ts`) is drawn by code and taken by any spec's `texture` parameter. Components
 * and layers are used in the content as `components: [{ type, … }]`, a node type as `type`. A **look** (`appearances.ts`)
 * is how an entity appears and disappears: the experience names it for a moment (`appearance: { reveal, found, lost }`).
 * Across nodes and pages a plugin uses the runtime (`plugin-runtime.ts`): `api.events` (core events of the
 * scene, scene events of the content), `api.vars` (variables in the scopes scene / stage / session / experience),
 * `api.progress` (what the reader unlocked – from the app) and `api.scene` (the current scene). A file of the experience's
 * Library – a shader (`.glsl`, `.frag`, `.vert`, `.wgsl`) – comes in as text through `api.loadText(url)` (#186), a
 * model the author picked in a `model` parameter through `api.loadModel(url)` (#263).
 * A **feedback buffer** (`feedback.ts`) is a fragment shader the engine runs once per frame into a render target
 * of its own (`api.feedback`), its own previous frame as `iChannel0` – a fluid on the GPU without the renderer in the
 * plugin's hands; `api.shadertoy(source)` turns a Shadertoy pass (`mainImage`) into its fragment.
 *
 * Registrations are kept per plugin id: installing a plugin again (the editor's Run) replaces what it registered
 * before; an exception in `register` leaves the previous registration in place. The reader loads the plugins the
 * configuration declares (`loadPlugins`), compiled by the content build; the editor also installs modules it
 * transpiled itself (`installPlugin`) – unchecked code. Both run in the editor's own page (#340, 5.0 – the stage frame
 * of #234 is gone): plugin code there acts with the signed-in editor's session, so the editor asks before it runs code
 * someone else saved last. Leaving the workspace clears the registry (`resetPlugins`).
 */

/** A spec's parameters as its handler gets them: typed from the spec (`ParamsOf`), a texture parameter as a three.js texture or null */
export type PluginParams<S> = ParamsOf<S, THREE.Texture | null>;

/**
 * What a component's `apply` gets: the node, its parameters resolved, the node's objects, the target, three.js, and `emit`.
 * `P`, `E`: the parameters and the event names its spec declares (inferred by `registerComponent`)
 */
export interface ComponentContext<P = Record<string, unknown>, E extends string = string> {
  node: EntityData;
  /** The component's parameters with the spec's defaults filled in */
  params: P;
  /** The node's own object (a model's scene, a video's plane) – the placement group for a node without one */
  object: Object3D;
  /** The placement group around the node (its position / rotation / scale relative to the parent) */
  placed: Object3D;
  /** The placement groups of the node's children, in content order (empty for a leaf) */
  children: Object3D[];
  target: Target;
  /** The app's three.js (one instance – plugins never import three themselves) */
  three: typeof THREE;
  /**
   * Emit one of the events its spec declares (`events`) on the bus, with the target's id (and the payload's fields) – a
   * rule listens to it by name. An event the spec does not declare is not sent (a warning)
   */
  emit(event: E, payload?: Record<string, unknown>): void;
}

/** The hooks a component returns; every one is optional. `P`, `A`: its spec's parameters and actions */
export interface ComponentInstance<P = Record<string, unknown>, A extends string = string> {
  /** Every rendered frame while the scene runs (seconds since the last frame) */
  update?(delta: number): void;
  onFound?(): void;
  onLost?(): void;
  /** The target's first find (the app's progress decides; the scene is told through `celebrate`) */
  onUnlocked?(): void;
  /** A tap on the entity */
  onTap?(): void;
  /** The scene paused (consultation, overlays) */
  onPause?(): void;
  /** Free what the component created */
  dispose?(): void;
  /** Apply other parameter values in place (the editor's live edit); false = the node is built anew */
  tune?(params: P): boolean;
  /**
   * A rule's verb on the node, one of the spec's `actions` (`play: door` reaches a component of the node `door` that
   * declares `play`) – beside what the engine does itself (a model's clip, the node's visibility)
   */
  onAction?(action: A): void;
}

export type ComponentApply<P = Record<string, unknown>, A extends string = string, E extends string = string> =
  (context: ComponentContext<P, E>) => ComponentInstance<P, A> | null | void;

/**
 * A material layer: the body of a GLSL function `vec4 (vec4 rgba, vec2 uv, float time, <the parameters>)`, run on
 * every pixel of the node's material after its own shading, in linear RGB. The spec's parameters arrive as
 * arguments in their order: number → float, boolean → float (0 / 1), color → vec3, choice → int (the index in its
 * options); string parameters are not passed. `time` is the seconds since the node was built.
 */
export interface LayerDefinition {
  glsl: string;
}

/** What a node type's `build` gets: the node, its parameters resolved, its file loaded, the target, three.js, `emit` */
export interface NodeTypeContext<P = Record<string, unknown>, E extends string = string> {
  node: EntityData;
  /** The node type's parameters (from the node's `params`) with the spec's defaults filled in */
  params: P;
  /**
   * The node's file, loaded – a model's `scene` and `animations`, a video's `element` and `texture`, an image's
   * `texture` – or undefined (a type without `src`, or the file did not load). It is shared with every node that uses
   * the same file: clone what you change (`asset.scene.clone()`), never dispose it.
   */
  asset?: LoadedAsset;
  target: Target;
  /** The app's three.js (one instance – plugins never import three themselves) */
  three: typeof THREE;
  /** Emit one of the events its spec declares, as a component's `emit` */
  emit(event: E, payload?: Record<string, unknown>): void;
}

/**
 * What a node type's `build` returns: the node's own three.js object – the engine puts it into the node's placement
 * group, next to its children – and the hooks it needs (every one optional, as a component's). When the node carries a
 * material it replaces the material of every mesh of the object, except a mesh with `userData.keepMaterial =
 * true`.
 */
export interface NodeTypeInstance<P = Record<string, unknown>, A extends string = string> {
  object: Object3D;
  update?(delta: number): void;
  onFound?(): void;
  onLost?(): void;
  onUnlocked?(): void;
  onTap?(): void;
  onPause?(): void;
  /** Free what the build created (geometries, materials, textures of its own – not the shared asset) */
  dispose?(): void;
  /** Apply other parameter values in place (the editor's live edit); false = the node is built anew */
  tune?(params: P): boolean;
  /** A rule's verb on the node, one of the spec's `actions` (as a component's) */
  onAction?(action: A): void;
}

export type NodeTypeBuild<P = Record<string, unknown>, A extends string = string, E extends string = string> =
  (context: NodeTypeContext<P, E>) => NodeTypeInstance<P, A> | null | void;

export interface PluginApi {
  /** `PLUGIN_API_VERSION` */
  readonly version: string;
  readonly three: typeof THREE;
  /**
   * The parameter builders (API 3.1, `p.number({ min, max, slider? })`, `p.choice(["a", "b"])` …): they write the plain
   * parameter objects, and the register functions infer the handler's `params` from them
   */
  readonly p: ParamBuilders;
  /**
   * A component: its spec (parameters, and optionally `actions`, `events`, `uses`) and `apply`, which gets the parameters
   * typed from the spec (`params.speed: number`), `emit` for the declared events and returns hooks (`onAction` for the
   * declared actions)
   */
  registerComponent<const S extends ComponentSpec>(type: string, spec: S, apply: ComponentApply<PluginParams<S>, ActionsOf<S>, EventsOf<S>>): void;
  registerLayer<const S extends ComponentSpec>(type: string, spec: S, layer: LayerDefinition): void;
  /**
   * A node type of the plugin's own: `build` makes its three.js object with `api.three` (Points, a Sprite,
   * a Line, a mesh with its own material); the engine places it like every node – placement, children, components and
   * layers come with it. The spec declares its parameters (rows in the Entity panel, values in the node's `params`)
   * and the file its `src` takes (`glb`, `video`, `image` or `none`).
   */
  registerNodeType<const S extends NodeTypeSpec>(type: string, spec: S, build: NodeTypeBuild<PluginParams<S>, ActionsOf<S>, EventsOf<S>>): void;
  /**
   * A post-processing pass of the plugin's own: its spec as the built-in passes have it, the factory builds the
   * three.js pass (`PassContext`: the resolved parameters, `shaderPass`, the canvas size). The content names it in
   * `passes: [{ type, … }]`; the plugin's pass leaves the registry when the plugin is uninstalled.
   */
  registerPass<const S extends PassSpec>(type: string, spec: S, factory: PassFactory<PluginParams<S>>): void;
  /**
   * A whole material: `build` makes a three.js material – a `ShaderMaterial` with GLSL of its own, a
   * `MeshStandardMaterial` with a procedural texture – and may return `{ material, update, tune, dispose }`. A node names
   * it as `material: { type, params }`; it replaces the material of the node's meshes (a model's, a plane's, a plugin
   * node's), the node's layers still add on top. `time` in the context is a uniform the engine advances.
   */
  registerMaterial<const S extends MaterialSpec>(type: string, spec: S, build: MaterialBuild<PluginParams<S>>): void;
  /**
   * A procedural texture: `draw` makes a three.js texture (a `CanvasTexture`, a `DataTexture`) and may
   * return `{ texture, update, dispose }` to animate it. Any spec's `texture` parameter takes it by its type – a
   * material's, a layer's, a component's, a node type's.
   */
  registerTexture<const S extends TextureSpec>(type: string, spec: S, draw: TextureDraw<ParamsOf<S>>): void;
  /**
   * A look of the plugin's own: how an entity appears and disappears. The factory runs once per animation with the
   * entity, its anchor, the resolved parameters and the moment (`AppearanceContext`) and returns
   * `{ duration, update(visible), dispose? }` – `visible` runs from 0 to 1 on `reveal` and `found`, from 1 to 0 on
   * `lost`. The experience names it by its type in `appearance: { reveal, found, lost }`, in a built-in look's place.
   */
  registerAppearance<const S extends AppearanceSpec>(type: string, spec: S, factory: AppearanceFactory<ParamsOf<S>>): void;
  /** Core events of the scene (read-only) and scene events of the content */
  readonly events: PluginEvents;
  /** Variables in the scopes scene, stage, session and experience */
  readonly vars: PluginVars;
  /** The reader's progress: unlocked, consulted, completion (read-only) */
  readonly progress: PluginProgress;
  /** The current scene */
  readonly scene: PluginScene;
  /**
   * A text file: a shader of the experience's Library by its URL (the Library shows it; a replace gives a new URL and
   * rewrites it in the sources that name it), or a content path (`/assets/content/…`). One request per URL however often
   * it is asked for; rejects on an HTTP error. A layer's GLSL is a string at register time – load files where a build
   * runs: a material's `build`, a pass's factory, a node type's `build` (set the shader when it arrives, `needsUpdate`).
   */
  loadText(url: string): Promise<string>;
  /**
   * A model file (`.glb`, `.gltf`): a model of the experience's Library by its URL, or a content path (`/assets/content/…`) –
   * the value of a `model` parameter, which the author picks in the editor. One request per URL however often it is
   * asked for; rejects on an HTTP error or a file that is no model. The model is shared by everyone who loads the same
   * file: clone what you change (`model.scene.clone()`, a geometry's `clone()`), never dispose it. It loads after the
   * build that asked for it returned – apply it when it arrives, and not at all once the node is disposed.
   */
  loadModel(url: string): Promise<PluginModel>;
  /**
   * A GPU feedback buffer: `fragment` runs once per frame into a render target the engine keeps (two, swapped
   * every frame, while it reads its own previous frame – the channel `"self"`, the default `iChannel0`), before the scene
   * renders, with `iTime`, `iTimeDelta`, `iFrame`, `iResolution` and `iChannel0…3` set. Show `texture` – or put `uniform`
   * into a ShaderMaterial's uniforms, it follows every swap. Made in a node's build (a node type's, a component's, a
   * material's) it belongs to the node: it pauses while the node is hidden or its page is not in view, and goes with it.
   * Half-float targets by default, float on request – 8 bits where the device renders into neither (a warning).
   */
  feedback(options: FeedbackOptions): Feedback;
  /**
   * A Shadertoy pass as `feedback`'s fragment: its `mainImage(out vec4, in vec2)` source unchanged, called with
   * Shadertoy's `fragCoord`; `iTime`, `iFrame`, `iResolution`, `iChannel0…3` are the buffer's. Throws without `mainImage`.
   */
  shadertoy(source: string): string;
  /** To the console, prefixed with the plugin's id */
  log(...args: unknown[]): void;
}

/** A model `api.loadModel` loaded: its scene and its animation clips */
export interface PluginModel {
  scene: THREE.Group;
  animations: THREE.AnimationClip[];
}

/** What a compiled plugin module exports */
export interface PluginModule {
  register?: (api: PluginApi) => void;
  default?: (api: PluginApi) => void;
}

interface ComponentRegistration { plugin: string; spec: ComponentSpec; apply: ComponentApply }
interface LayerRegistration { plugin: string; spec: ComponentSpec; layer: LayerDefinition }
interface PassRegistrationStaged { spec: PassSpec; factory: PassFactory }
interface AppearanceStaged { spec: AppearanceSpec; factory: AppearanceFactory }
interface NodeTypeRegistration { plugin: string; spec: NodeTypeSpec; build: NodeTypeBuild }
interface MaterialStaged { spec: MaterialSpec; build: MaterialBuild }
interface TextureStaged { spec: TextureSpec; draw: TextureDraw }

const components = new Map<string, ComponentRegistration>();
const layers = new Map<string, LayerRegistration>();
const nodeTypes = new Map<string, NodeTypeRegistration>();
/** Plugins installed, by id → the component and layer types they registered */
const installed = new Map<string, string[]>();
/** Plugins installed, by id → the node types they registered (a namespace of its own) */
const installedNodeTypes = new Map<string, string[]>();
/** Plugins installed, by id → the materials and the procedural textures they registered */
const installedMaterials = new Map<string, string[]>();
const installedTextures = new Map<string, string[]>();
/** The compiled modules loaded, by URL (a scene switch never loads a plugin twice) */
const loaded = new Map<string, Promise<void>>();
/** Plugins whose code the editor installed itself (a Run, the saved sources' auto-load, #269): the content's module of the same id is not loaded over them */
const keptPlugins = new Set<string>();

/**
 * The editor installed this plugin from its own source (#269): `loadPlugins` leaves it be – the content's compiled
 * module of the same id (an older build of it) is not installed over it, whichever arrives first
 */
export const keepPlugin = (id: string): void => {
  keptPlugins.add(id);
};
/** The text files plugins loaded (`api.loadText`), by resolved URL – a failed one is asked for again next time */
const texts = new Map<string, Promise<string>>();

/** `api.loadText`: a text file by URL or content path, one request per URL */
export const loadText = (url: string, fetcher: typeof fetch = (input, init) => fetch(input, init)): Promise<string> => {
  const resolved = contentUrl(url);
  let loading = texts.get(resolved);
  if (!loading) {
    loading = fetcher(resolved).then(response => {
      if (!response.ok) throw new Error(`${response.status} ${response.statusText} (${resolved})`);
      return response.text();
    });
    loading.catch(() => texts.delete(resolved));
    texts.set(resolved, loading);
  }
  return loading;
};

/** The model files plugins loaded (`api.loadModel`), by resolved URL – a failed one is asked for again next time */
const models = new Map<string, Promise<PluginModel>>();

/** `api.loadModel`: a model file by URL or content path, one request per URL */
export const loadModel = (url: string, loader: (src: string, assetType: "glb" | "gltf") => Promise<PluginModel> = loadGltf): Promise<PluginModel> => {
  const resolved = contentUrl(url);
  let loading = models.get(resolved);
  if (!loading) {
    const file = resolved.split(/[?#]/)[0];
    loading = /\.(glb|gltf)$/i.test(file)
      ? loader(resolved, /\.gltf$/i.test(file) ? "gltf" : "glb")
      : Promise.reject(new Error(`a model is a .glb or .gltf file (${url || "no file"})`));
    loading.catch(() => models.delete(resolved));
    models.set(resolved, loading);
  }
  return loading;
};

export class PluginError extends Error {
  constructor(public readonly plugin: string, message: string) {
    super(message);
    this.name = "PluginError";
  }
}

/** Every component, layer and pass type this plugin registered leaves the registries, its listeners with them */
export const uninstallPlugin = (id: string): void => {
  dropPluginListeners(id);
  removeRegistrations(id);
  disposePluginFeedbacks(id);
  revision += 1;
};

const removeRegistrations = (id: string): void => {
  (installed.get(id) ?? []).forEach(type => {
    if (components.get(type)?.plugin === id) components.delete(type);
    if (layers.get(type)?.plugin === id) layers.delete(type);
    if (components.get(type)?.plugin !== id && layers.get(type)?.plugin !== id) unregisterComponentSpec(type);
  });
  (installedNodeTypes.get(id) ?? []).forEach(type => {
    if (nodeTypes.get(type)?.plugin !== id) return;
    nodeTypes.delete(type);
    unregisterNodeTypeSpec(type);
  });
  (installedMaterials.get(id) ?? []).forEach(type => {
    if (materialRegistration(type)?.plugin !== id) return;
    deleteMaterialRegistration(type);
    unregisterMaterialSpec(type);
  });
  (installedTextures.get(id) ?? []).forEach(type => {
    if (textureRegistration(type)?.plugin !== id) return;
    deleteTextureRegistration(type);
    unregisterTextureSpec(type);
  });
  unregisterPassesOf(id);
  unregisterAppearancesOf(id);
  installed.delete(id);
  installedNodeTypes.delete(id);
  installedMaterials.delete(id);
  installedTextures.delete(id);
};

/**
 * Run a plugin module's `register(api)`: its registrations replace the plugin's previous ones. Throws a
 * `PluginError` (and keeps the previous registrations) when the module exports no register function, a spec is
 * invalid, a type is taken by another plugin, or `register` throws.
 */
export const installPlugin = (id: string, module: PluginModule): void => {
  const register = module.register ?? module.default;
  if (typeof register !== "function") throw new PluginError(id, `${id}: the module exports no register(api) function`);
  const staged: {
    components: Map<string, ComponentRegistration>; layers: Map<string, LayerRegistration>; passes: Map<string, PassRegistrationStaged>;
    nodeTypes: Map<string, NodeTypeRegistration>; materials: Map<string, MaterialStaged>; textures: Map<string, TextureStaged>;
    appearances: Map<string, AppearanceStaged>;
  } = {
    components: new Map(),
    layers: new Map(),
    passes: new Map(),
    nodeTypes: new Map(),
    materials: new Map(),
    textures: new Map(),
    appearances: new Map(),
  };
  // The listeners this run subscribes replace the previous run's once register succeeded
  const run = nextRun();
  const check = (type: string, spec: ComponentSpec) => {
    const problems = componentSpecProblems(type, spec);
    const owner = components.get(type)?.plugin ?? layers.get(type)?.plugin;
    if (owner && owner !== id) problems.push(`"${type}" is registered by the plugin ${owner}`);
    if (staged.components.has(type) || staged.layers.has(type)) problems.push(`"${type}" is registered twice`);
    if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
  };
  const api: PluginApi = {
    version: PLUGIN_API_VERSION,
    three: THREE,
    p: PARAM_BUILDERS,
    ...runtimeApi(id, run),
    registerComponent: (type, spec, apply) => {
      check(type, spec);
      if (typeof apply !== "function") throw new PluginError(id, `${id}: ${type} needs an apply(context) function`);
      staged.components.set(type, { plugin: id, spec, apply: apply as ComponentApply });
    },
    registerLayer: (type, spec, layer) => {
      check(type, spec);
      if (typeof layer?.glsl !== "string" || !layer.glsl.trim()) throw new PluginError(id, `${id}: the layer ${type} needs its glsl`);
      staged.layers.set(type, { plugin: id, spec, layer });
    },
    registerPass: (type, spec, factory) => {
      const problems = componentSpecProblems(type, spec);
      const owner = passRegistration(type)?.owner;
      if (owner && owner !== id) problems.push(`the pass "${type}" is registered by ${owner}`);
      if (staged.passes.has(type)) problems.push(`the pass "${type}" is registered twice`);
      if (typeof factory !== "function") problems.push(`the pass "${type}" needs a factory`);
      if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
      staged.passes.set(type, { spec, factory: factory as PassFactory });
    },
    registerNodeType: (type, spec, build) => {
      const problems = nodeTypeSpecProblems(type, spec);
      const owner = nodeTypes.get(type)?.plugin;
      if (owner && owner !== id) problems.push(`the node type "${type}" is registered by the plugin ${owner}`);
      if (staged.nodeTypes.has(type)) problems.push(`the node type "${type}" is registered twice`);
      if (typeof build !== "function") problems.push(`the node type "${type}" needs a build(context) function`);
      if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
      staged.nodeTypes.set(type, { plugin: id, spec, build: build as NodeTypeBuild });
    },
    registerMaterial: (type, spec, build) => {
      const problems = materialSpecProblems(type, spec);
      const owner = materialRegistration(type)?.plugin;
      if (owner && owner !== id) problems.push(`the material "${type}" is registered by the plugin ${owner}`);
      if (staged.materials.has(type)) problems.push(`the material "${type}" is registered twice`);
      if (typeof build !== "function") problems.push(`the material "${type}" needs a build(context) function`);
      if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
      staged.materials.set(type, { spec, build: build as MaterialBuild });
    },
    registerTexture: (type, spec, draw) => {
      const problems = textureSpecProblems(type, spec);
      const owner = textureRegistration(type)?.plugin;
      if (owner && owner !== id) problems.push(`the texture "${type}" is registered by the plugin ${owner}`);
      if (staged.textures.has(type)) problems.push(`the texture "${type}" is registered twice`);
      if (typeof draw !== "function") problems.push(`the texture "${type}" needs a draw(context) function`);
      if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
      staged.textures.set(type, { spec, draw: draw as TextureDraw });
    },
    registerAppearance: (type, spec, factory) => {
      const problems = componentSpecProblems(type, spec).filter(problem => !problem.endsWith("is a built-in component"));
      if (isBuiltInAppearanceType(type)) problems.push(`"${type}" is a built-in look`);
      const owner = appearanceRegistration(type)?.owner;
      if (owner && owner !== id) problems.push(`the look "${type}" is registered by ${owner}`);
      if (staged.appearances.has(type)) problems.push(`the look "${type}" is registered twice`);
      if (typeof factory !== "function") problems.push(`the look "${type}" needs a factory`);
      if (problems.length) throw new PluginError(id, `${id}: ${problems.join("; ")}`);
      staged.appearances.set(type, { spec, factory: factory as AppearanceFactory });
    },
    loadText: url => loadText(url),
    loadModel: url => loadModel(url),
    feedback: options => createFeedback(id, options),
    shadertoy: source => shadertoyFragment(source),
    log: (...args) => console.info(`[plugin ${id}]`, ...args),
  };
  try {
    register(api);
  } catch (error) {
    dropPluginListeners(id, { onlyRun: run });
    if (error instanceof PluginError) throw error;
    throw new PluginError(id, `${id}: register failed – ${error instanceof Error ? error.message : String(error)}`);
  }
  dropPluginListeners(id, { keepRun: run });
  removeRegistrations(id);
  staged.components.forEach((registration, type) => {
    components.set(type, registration);
    registerComponentSpec(type, registration.spec);
  });
  staged.layers.forEach((registration, type) => {
    layers.set(type, registration);
    registerComponentSpec(type, registration.spec);
  });
  staged.passes.forEach(({ spec, factory }, type) => registerPass(type, spec, factory, id));
  staged.appearances.forEach(({ spec, factory }, type) => registerAppearance(type, spec, factory, id));
  staged.nodeTypes.forEach((registration, type) => {
    failures.delete(type);
    nodeTypes.set(type, registration);
    registerNodeTypeSpec(type, registration.spec);
  });
  staged.materials.forEach(({ spec, build }, type) => {
    setMaterialRegistration(type, { plugin: id, spec, build });
    registerMaterialSpec(type, spec);
  });
  staged.textures.forEach(({ spec, draw }, type) => {
    setTextureRegistration(type, { plugin: id, spec, draw });
    registerTextureSpec(type, spec);
  });
  installed.set(id, [...staged.components.keys(), ...staged.layers.keys()]);
  installedNodeTypes.set(id, [...staged.nodeTypes.keys()]);
  installedMaterials.set(id, [...staged.materials.keys()]);
  installedTextures.set(id, [...staged.textures.keys()]);
  revision += 1;
};

/** The ids of the plugins installed right now */
export const installedPlugins = (): string[] => [...installed.keys()];

/** The component and layer types a plugin registered */
export const pluginTypes = (id: string): string[] => [...(installed.get(id) ?? [])];

/** The node types a plugin registered */
export const pluginNodeTypesOf = (id: string): string[] => [...(installedNodeTypes.get(id) ?? [])];

/** The materials and the procedural textures a plugin registered */
export const pluginMaterialsOf = (id: string): string[] => [...(installedMaterials.get(id) ?? [])];
export const pluginTexturesOf = (id: string): string[] => [...(installedTextures.get(id) ?? [])];

export const nodeTypeRegistration = (type: string): NodeTypeRegistration | undefined => nodeTypes.get(type);

/** The node types whose last build failed, with why (a build that works, or the type registered anew, clears it) */
const failures = new Map<string, string>();

/** Counts every change of the registry: a plugin installed or uninstalled, a node type's build failing or building again */
let revision = 0;
/** A node type's failure, counted only when it changes */
const setFailure = (type: string, why: string | null): void => {
  if ((failures.get(type) ?? null) === why) return;
  if (why === null) failures.delete(type);
  else failures.set(type, why);
  revision += 1;
};

/**
 * The registry's revision: it changes with every installed or uninstalled plugin and every node type's build that failed
 * or built again – the editor reads its panels' specs again only then (#340)
 */
export const pluginRegistryRevision = (): number => revision;

/** Why the last build of a plugin's node type failed, undefined when it built (the editor shows it with the type's code) */
export const nodeTypeFailure = (type: string): string | undefined => failures.get(type);

/**
 * Build a node of a plugin's type: the registration's `build` with the node's parameters resolved and its file
 * loaded. Null when the type is nobody's, `build` returns nothing or throws (logged – the node renders nothing and
 * keeps its children). `tune` applies another node of the same type and file in place: the build's own `tune`, or –
 * without one – only when the parameters did not change.
 */
export const buildPluginNode = (
  node: EntityData,
  target: Target,
  asset: (id: string) => LoadedAsset | undefined,
): (Omit<NodeTypeInstance, "tune"> & { tune(next: EntityData): boolean; act(verb: string): boolean }) | null => {
  const registration = nodeTypes.get(node.type);
  if (!registration) return null;
  const where = `node type ${node.type} on ${target.id}`;
  const params = withinLimits(registration.spec.params, resolveNodeParams(registration.spec, node.params), null, where);
  const file = node.assets[0];
  // Texture parameters arrive as three.js textures
  const textures = resolveTextures(registration.spec.params, params, `node type ${node.type} on ${target.id}`);
  let instance: NodeTypeInstance | null | void;
  // The feedback buffers the build makes belong to the node: they pause with it and go with it
  let feedbacks: OwnedFeedbacks | null = null;
  try {
    [instance, feedbacks] = collectFeedbacks(() => registration.build({ node, params: textures.values, asset: file ? asset(file.id) : undefined, target, three: THREE, emit: specEmitter(registration.plugin, registration.spec, target, node.type) }));
  } catch (error) {
    console.error(`[plugins] the node type ${node.type} (plugin ${registration.plugin}) failed on ${target.id}:`, error);
    setFailure(node.type, error instanceof Error ? `${error.name}: ${error.message}` : String(error));
    textures.dispose();
    return null;
  }
  if (!instance || !(instance.object instanceof THREE.Object3D)) {
    if (instance) {
      console.error(`[plugins] the node type ${node.type} (plugin ${registration.plugin}) returned no three.js object on ${target.id}`);
      setFailure(node.type, "its build returned no three.js object");
    }
    feedbacks?.dispose();
    textures.dispose();
    return null;
  }
  setFailure(node.type, null);
  const built = instance;
  const owned = feedbacks;
  owned?.own(built.object);
  let applied = params;
  return {
    ...built,
    act: verb => answers(registration.spec, verb) && !!built.onAction && (call(() => built.onAction!(verb), where), true),
    update: delta => {
      textures.update(delta);
      built.update?.(delta);
    },
    dispose: () => {
      built.dispose?.();
      owned?.dispose();
      textures.dispose();
    },
    tune: next => {
      if (next.type !== node.type || (next.assets[0]?.src ?? "") !== (file?.src ?? "")) return false;
      const values = withinLimits(registration.spec.params, resolveNodeParams(registration.spec, next.params), applied, where);
      // Another texture: built anew
      if (!sameTextures(registration.spec.params, values, applied)) return false;
      const same = Object.keys(values).every(key => JSON.stringify(values[key]) === JSON.stringify(applied[key]));
      const ok = same || (built.tune?.(withTextures(registration.spec.params, values, textures.values)) ?? false);
      if (ok) applied = values;
      return ok;
    },
  };
};

/** New values with the texture parameters' textures of a resolved set (a live edit keeps the textures) */
const withTextures = (specParams: NodeTypeSpec["params"], values: Record<string, unknown>, resolved: Record<string, unknown>): Record<string, unknown> => {
  const next = { ...values };
  Object.entries(specParams).forEach(([name, param]) => { if (param.kind === "texture") next[name] = resolved[name]; });
  return next;
};

/** Load and install the plugins a configuration declares; a plugin that fails is logged and skipped (RULES #16) */
export const loadPlugins = async (
  plugins: readonly PluginData[],
  importer: (url: string) => Promise<unknown> = url => import(/* @vite-ignore */ url),
): Promise<void> => {
  await Promise.all(plugins.map(plugin => {
    // The editor's stage runs the Code tab's saved source of this id instead (#269): the compiled module – maybe built
    // against an older plugin API – is neither checked nor loaded
    if (keptPlugins.has(plugin.id)) return Promise.resolve();
    if (!pluginApiCompatible(plugin.api)) {
      console.error(`[plugins] ${plugin.id} was built against plugin API ${plugin.api}, this engine runs ${PLUGIN_API_VERSION} – not loaded`);
      return Promise.resolve();
    }
    let loading = loaded.get(plugin.src);
    if (!loading) {
      loading = importer(plugin.src)
        .then(module => {
          if (!keptPlugins.has(plugin.id)) installPlugin(plugin.id, module as PluginModule);
        })
        .catch(error => {
          loaded.delete(plugin.src);
          console.error(`[plugins] ${plugin.id} (${plugin.src}) could not be loaded:`, error);
        });
      loaded.set(plugin.src, loading);
    }
    return loading;
  }));
};

/** The components of a node that a plugin registered (not the built-in ones), with their parameters resolved */
export const pluginComponentsOf = (node: EntityData, previous?: ReadonlyArray<Record<string, unknown>>): Array<{ data: ComponentData; registration: ComponentRegistration; params: Record<string, unknown> }> =>
  (node.components ?? []).flatMap(data => {
    const registration = components.get(data.type);
    if (!registration) return [];
    const at = (node.components ?? []).filter(c => components.has(c.type)).indexOf(data);
    return [{ data, registration, params: withinLimits(registration.spec.params, resolveParams(registration.spec, data), previous?.[at] ?? null, `${data.type} on ${node.name || node.type}`) }];
  });

/**
 * Values outside a number parameter's hard limits (`min`, `max`, `integer` – the guard and the editor refuse them) are not
 * applied: the value applied before stays (the default when there is none yet), with a warning
 */
export const withinLimits = (
  specParams: Record<string, ComponentSpec["params"][string]>, values: Record<string, unknown>, previous: Record<string, unknown> | null, where: string,
): Record<string, unknown> => {
  let checked = values;
  for (const [name, param] of Object.entries(specParams)) {
    if (param.kind !== "number") continue;
    const problem = numberProblem(param, values[name]);
    if (!problem) continue;
    const kept = previous && !numberProblem(param, previous[name]) ? previous[name] : param.default;
    console.warn(`[plugins] ${where}: ${name} ${problem} – keeps ${JSON.stringify(kept)}`);
    checked = { ...checked, [name]: kept };
  }
  return checked;
};

/** Does a spec answer a rule's verb (`actions`)? */
const answers = (spec: { actions?: readonly string[] }, verb: string): boolean => !!spec.actions?.includes(verb);

/** Run a plugin's hook; what it throws is logged, never the scene's */
const call = (hook: () => void, where: string): void => {
  try {
    hook();
  } catch (error) {
    console.error(`[plugins] ${where}:`, error);
  }
};

/** A handler's `emit`: one of its spec's events on the bus, with the target's id; an undeclared one is not sent */
const specEmitter = (plugin: string, spec: { events?: readonly string[] }, target: Target, type: string) =>
  (event: string, payload?: Record<string, unknown>): void => {
    if (!spec.events?.includes(event)) {
      console.warn(`[plugins] ${type} (plugin ${plugin}) emits "${event}", which its spec does not declare (events: ${spec.events?.join(", ") || "none"}) – not sent`);
      return;
    }
    runtimeApi(plugin, 0).events.emit(event, { ...payload, targetId: target.id });
  };

/** The material layers of a node, in order, with their parameters resolved */
export const layersOf = (node: EntityData): Array<{ data: ComponentData; registration: LayerRegistration; params: Record<string, unknown> }> =>
  (node.components ?? []).flatMap(data => {
    const registration = layers.get(data.type);
    return registration ? [{ data, registration, params: resolveParams(registration.spec, data) }] : [];
  });

/** A component type nobody registered (built-in or plugin): the node renders without it */
export const unknownComponentTypes = (node: EntityData, builtIn: readonly string[]): string[] =>
  (node.components ?? []).map(c => c.type).filter(type => !builtIn.includes(type) && !components.has(type) && !layers.has(type));

/**
 * Apply a node's plugin components: one instance per component, as `EntityInstance`-like parts the entity registry
 * adds to the node's hooks. `tune` applies new parameter values to every instance in place when the components
 * stay the same types in the same order; else the node is built anew.
 */
export const applyComponents = (
  context: Omit<ComponentContext, "params" | "three" | "emit">,
): { parts: ComponentInstance[]; types: string[]; tune(node: EntityData): boolean; act(verb: string): boolean } => {
  const applied = pluginComponentsOf(context.node);
  // Texture parameters arrive as three.js textures; each component's go with it
  const resolved = applied.map(({ registration, params, data }) => resolveTextures(registration.spec.params, params, `${data.type} on ${context.target.id}`));
  const parts = applied.map(({ registration, data }, i): ComponentInstance => {
    const textures = resolved[i];
    let part: ComponentInstance;
    let feedbacks: OwnedFeedbacks | null = null;
    try {
      [part, feedbacks] = collectFeedbacks(() => registration.apply({ ...context, params: textures.values, three: THREE, emit: specEmitter(registration.plugin, registration.spec, context.target, data.type) }) ?? {});
      // The component's feedback buffers pause with the node's object and go with it
      feedbacks.own(context.object);
    } catch (error) {
      console.error(`[plugins] ${data.type} (plugin ${registration.plugin}) failed on ${context.target.id}:`, error);
      part = {};
    }
    const owned = feedbacks;
    if (!Object.values(textures.values).some(value => value instanceof THREE.Texture) && !owned?.count) return part;
    return {
      ...part,
      update: delta => {
        textures.update(delta);
        part.update?.(delta);
      },
      dispose: () => {
        part.dispose?.();
        owned?.dispose();
        textures.dispose();
      },
    };
  });
  const types = applied.map(a => a.data.type);
  return {
    parts,
    types,
    // A rule's verb: every component of the node whose spec declares it (true when one took it)
    act: verb => applied.reduce((took, { registration, data }, i) => {
      const hook = parts[i].onAction;
      if (!answers(registration.spec, verb) || !hook) return took;
      call(() => hook(verb), `${data.type} on ${context.target.id}`);
      return true;
    }, false),
    tune: node => {
      const next = pluginComponentsOf(node, applied.map(a => a.params));
      if (next.length !== applied.length || next.some((c, i) => c.data.type !== types[i])) return false;
      if (next.some((c, i) => !sameTextures(c.registration.spec.params, c.params, applied[i].params))) return false;
      return next.every((c, i) => parts[i].tune?.(withTextures(c.registration.spec.params, c.params, resolved[i].values))
        ?? Object.keys(c.params).every(key => JSON.stringify(c.params[key]) === JSON.stringify(applied[i].params[key])));
    },
  };
};

/** One plugin's registrations as data – its specs by kind and type, no code */
export interface PluginRegistrations {
  components: Record<string, ComponentSpec>;
  layers: Record<string, ComponentSpec>;
  nodeTypes: Record<string, NodeTypeSpec>;
  materials: Record<string, MaterialSpec>;
  textures: Record<string, TextureSpec>;
  passes: Record<string, PassSpec>;
  appearances: Record<string, AppearanceSpec>;
}

/** The plugin registry as data: every installed plugin's registrations and the node types whose last build failed */
export interface PluginRegistrySnapshot {
  plugins: Record<string, PluginRegistrations>;
  failures: Record<string, string>;
}

/** A spec as plain data (what survives JSON) */
const plain = <T>(spec: T): T => JSON.parse(JSON.stringify(spec)) as T;

/** The registry as data: what a publish declares of the plugins that ran on the editor's stage (`publish-plugins.ts`) */
export const pluginRegistrySnapshot = (): PluginRegistrySnapshot => {
  const plugins: Record<string, PluginRegistrations> = {};
  installed.forEach((types, id) => {
    const entry: PluginRegistrations = { components: {}, layers: {}, nodeTypes: {}, materials: {}, textures: {}, passes: {}, appearances: {} };
    types.forEach(type => {
      const component = components.get(type);
      if (component?.plugin === id) entry.components[type] = plain(component.spec);
      const layer = layers.get(type);
      if (layer?.plugin === id) entry.layers[type] = plain(layer.spec);
    });
    (installedNodeTypes.get(id) ?? []).forEach(type => {
      const registration = nodeTypes.get(type);
      if (registration?.plugin === id) entry.nodeTypes[type] = plain(registration.spec);
    });
    (installedMaterials.get(id) ?? []).forEach(type => {
      const registration = materialRegistration(type);
      if (registration?.plugin === id) entry.materials[type] = plain(registration.spec);
    });
    (installedTextures.get(id) ?? []).forEach(type => {
      const registration = textureRegistration(type);
      if (registration?.plugin === id) entry.textures[type] = plain(registration.spec);
    });
    registeredPassTypes().forEach(type => {
      const registration = passRegistration(type);
      if (registration?.owner === id) entry.passes[type] = plain(registration.spec);
    });
    registeredAppearanceTypes().forEach(type => {
      const registration = appearanceRegistration(type);
      if (registration?.owner === id) entry.appearances[type] = plain(registration.spec);
    });
    plugins[id] = entry;
  });
  return { plugins, failures: Object.fromEntries(failures) };
};

/**
 * Every plugin leaves the registries, and the editor's marks go with them (#340): the plugins it installed itself
 * (`keepPlugin`), the compiled modules loaded, the text files read and the node types' build failures. The editor's
 * workspace calls it when it closes – the stage runs in the editor's page, and the next experience starts with an empty
 * registry, as a new page would.
 */
export const resetPlugins = (): void => {
  [...installed.keys(), ...installedNodeTypes.keys(), ...installedMaterials.keys(), ...installedTextures.keys()].forEach(id => uninstallPlugin(id));
  keptPlugins.clear();
  loaded.clear();
  texts.clear();
  failures.clear();
  revision += 1;
};
