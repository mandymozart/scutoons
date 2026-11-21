import {
  BufferGeometry, ClampToEdgeWrapping, Float32BufferAttribute, FloatType, HalfFloatType, LinearFilter, Mesh, NearestFilter,
  type Object3D, OrthographicCamera, RepeatWrapping, RGBAFormat, ShaderMaterial, type Texture, type TextureDataType, UnsignedByteType,
  Vector3, type WebGLRenderer, WebGLRenderTarget,
} from "three";
import { type Feedback, type FeedbackChannel, feedbackPrelude, feedbackProblems, type FeedbackOptions, type FeedbackType, supportedFeedbackType } from "./feedback-spec";

export * from "./feedback-spec";

/**
 * GPU feedback buffers for plugins (docs/plugins.md "A feedback buffer"): a fragment shader the engine
 * runs once per frame into a render target of its own, before the scene renders – a fluid, a reaction-diffusion, a
 * painting that remembers. A plugin never gets the renderer: it asks `api.feedback({ width, height, fragment, … })` and
 * gets a handle whose `texture` (and `uniform`, the same as a uniform that follows it) it shows on a material.
 *
 * - **Ping-pong**: a buffer that reads its own previous frame (the channel `"self"`, the default `iChannel0`) has two
 *   targets; every step reads one and writes the other, then they swap. A pass that does not read itself has one target.
 * - **Uniforms** the engine sets before each step: `iTime` (seconds the buffer has run), `iTimeDelta`, `iFrame` (steps
 *   done, 0 on the first), `iResolution` (`vec3(width, height, 1)`), `iChannel0…3` (the channels: `"self"`, another
 *   buffer – its current frame, or a three.js texture). The plugin's own uniforms join them by reference.
 * - **The fragment** is GLSL ES 3.0 (three.js compiles a `ShaderMaterial` as such: `texture`, `int` uniforms) after a prelude that declares those uniforms and `varying vec2 vUv` (0…1 across the
 *   buffer; `vUv * iResolution.xy` is the pixel's centre, as Shadertoy's `fragCoord`); it writes `gl_FragColor`.
 *   `shadertoyFragment` wraps a Shadertoy pass (`mainImage`) into one.
 * - **Precision**: `half` (default) and `float` targets where the device renders into them; otherwise the next lower one
 *   – float → half → 8 bits (`byte`, values clamped to 0…1) – with one warning. `type` on the handle says which it got.
 * - **Pause and ownership**: a buffer created while a node builds (a node type's `build`, a component's `apply`, a
 *   material's `build`) belongs to that node: it steps only while the node's object and every parent are visible in the
 *   scene (a lost target's anchor is hidden, another scene's nodes are gone) and is disposed with the node. One made
 *   elsewhere (in `register`, later in a hook) belongs to the plugin and goes when the plugin is uninstalled. One made in
 *   a post-processing pass's factory belongs to the pass: it steps while the pass runs in a chain
 *   that renders with its passes and goes when the chain drops or rebuilds the pass (`post/chain.ts`). A hidden
 *   page has no frames at all; `paused` on the handle stops one by hand.
 *
 * The view steps every live buffer in creation order (`stepFeedbacks`, from `ArView`'s loop – the reader's and the
 * editor's stage are the same engine), so a pass that reads another buffer created before it sees that buffer's new frame.
 */

const VERTEX = /* glsl */ `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = vec4(position.xy, 0.0, 1.0);
}
`;

export type FeedbackRenderer = Pick<WebGLRenderer, "render" | "setRenderTarget" | "getRenderTarget"> & {
  extensions: { has(name: string): boolean };
};

const DATA_TYPES: Record<FeedbackType, TextureDataType> = { half: HalfFloatType, float: FloatType, byte: UnsignedByteType };

/** One big triangle over the target (shared by every buffer, never disposed – a few bytes) */
const triangle = (() => {
  let geometry: BufferGeometry | null = null;
  return () => {
    if (!geometry) {
      geometry = new BufferGeometry();
      geometry.setAttribute("position", new Float32BufferAttribute([-1, 3, 0, -1, -1, 0, 3, -1, 0], 3));
      geometry.setAttribute("uv", new Float32BufferAttribute([0, 2, 0, 0, 2, 0], 2));
    }
    return geometry;
  };
})();
const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);

class FeedbackBuffer implements Feedback {
  readonly uniform: { value: Texture };
  readonly uniforms: Record<string, { value: unknown }>;
  readonly width: number;
  readonly height: number;
  paused = false;
  /** The node object it belongs to (null: the plugin's – it runs while the plugin is installed) */
  owner: Object3D | null = null;
  /** Held by the engine: a post-processing pass's buffer while its chain renders without passes (not `paused`, the plugin's) */
  held = false;
  disposed = false;
  private targets: WebGLRenderTarget[];
  private read = 0;
  private steps = 0;
  private time = 0;
  private readonly material: ShaderMaterial;
  private readonly mesh: Mesh;
  private readonly channels: FeedbackChannel[];
  private resolved: FeedbackType;
  private checked = false;

  constructor(readonly plugin: string, private readonly options: FeedbackOptions) {
    this.width = options.width;
    this.height = options.height;
    this.channels = options.channels ?? ["self"];
    this.resolved = options.type ?? "half";
    if (probe) this.resolve(probe);
    this.targets = this.makeTargets();
    this.uniform = { value: this.targets[0].texture };
    this.uniforms = {
      ...(options.uniforms ?? {}),
      iTime: { value: 0 },
      iTimeDelta: { value: 0 },
      iFrame: { value: 0 },
      iResolution: { value: new Vector3(this.width, this.height, 1) },
      ...Object.fromEntries(this.channels.map((_, i) => [`iChannel${i}`, { value: null }])),
    };
    this.material = new ShaderMaterial({
      uniforms: this.uniforms,
      vertexShader: VERTEX,
      fragmentShader: feedbackPrelude(this.channels.length) + options.fragment,
      depthTest: false,
      depthWrite: false,
    });
    this.mesh = new Mesh(triangle(), this.material);
    this.mesh.frustumCulled = false;
  }

  get texture(): Texture {
    return this.uniform.value;
  }

  get type(): FeedbackType {
    return this.resolved;
  }

  get frame(): number {
    return this.steps;
  }

  /** Reads its own previous frame: two targets */
  private get pingPong(): boolean {
    return this.channels.includes("self");
  }

  private makeTargets(): WebGLRenderTarget[] {
    const filter = this.options.filter === "nearest" ? NearestFilter : LinearFilter;
    const wrap = this.options.wrap === "repeat" ? RepeatWrapping : ClampToEdgeWrapping;
    const make = () => new WebGLRenderTarget(this.width, this.height, {
      type: DATA_TYPES[this.resolved], format: RGBAFormat, minFilter: filter, magFilter: filter, wrapS: wrap, wrapT: wrap,
      depthBuffer: false, stencilBuffer: false, generateMipmaps: false,
    });
    return this.pingPong ? [make(), make()] : [make()];
  }

  /** The device's precision, once: a lower one than asked is a warning (and new targets when they exist already) */
  resolve(renderer: Pick<FeedbackRenderer, "extensions">): void {
    if (this.checked) return;
    this.checked = true;
    const asked = this.resolved;
    const got = supportedFeedbackType(asked, name => renderer.extensions.has(name), this.options.filter !== "nearest");
    if (got === asked) return;
    console.warn(`[feedback] ${this.plugin}: ${asked} render targets are not supported here – ${got === "byte" ? "8 bits per channel (values clamp to 0…1)" : `${got} instead`}`);
    this.resolved = got;
    if (!this.targets) return;
    this.targets.forEach(target => target.dispose());
    this.targets = this.makeTargets();
    this.read = 0;
    this.uniform.value = this.targets[0].texture;
  }

  /** Visible in the scene: the owner and every parent up to a scene (a node not yet placed is not) */
  live(): boolean {
    if (this.paused || this.held || this.disposed) return false;
    if (!this.owner) return true;
    let object: Object3D | null = this.owner;
    while (object) {
      if (!object.visible) return false;
      if ((object as Object3D & { isScene?: boolean }).isScene) return true;
      object = object.parent;
    }
    return false;
  }

  /** One frame: the channels bound, the pass into the write target, the targets swapped */
  step(renderer: FeedbackRenderer, delta: number): void {
    this.resolve(renderer);
    const read = this.targets[this.read];
    const write = this.pingPong ? this.targets[1 - this.read] : read;
    this.channels.forEach((channel, i) => {
      this.uniforms[`iChannel${i}`].value = channel === "self" ? read.texture : channel instanceof FeedbackBuffer ? channel.texture : channel;
    });
    this.uniforms.iTime.value = this.time;
    this.uniforms.iTimeDelta.value = delta;
    this.uniforms.iFrame.value = this.steps;
    renderer.setRenderTarget(write);
    renderer.render(this.mesh, camera);
    if (this.pingPong) this.read = 1 - this.read;
    this.uniform.value = this.targets[this.read].texture;
    this.steps++;
    this.time += delta;
  }

  reset(): void {
    this.steps = 0;
    this.time = 0;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    buffers.delete(this);
    this.targets.forEach(target => target.dispose());
    this.material.dispose();
  }
}

/** Every buffer alive, in creation order */
const buffers = new Set<FeedbackBuffer>();
/** The renderer's extensions, once a view exists: a buffer made after it gets its precision at once */
let probe: Pick<FeedbackRenderer, "extensions"> | null = null;
/** The buffers made while a node builds (`collectFeedbacks`) */
let scope: FeedbackBuffer[] | null = null;

/** `api.feedback` of a plugin: a new buffer (throws on bad options – a build's error, logged by the engine) */
export const createFeedback = (plugin: string, options: FeedbackOptions): Feedback => {
  const problems = feedbackProblems(options);
  if (problems.length) throw new Error(`api.feedback: ${problems.join("; ")}`);
  const buffer = new FeedbackBuffer(plugin, options);
  buffers.add(buffer);
  scope?.push(buffer);
  return buffer;
};

/** The view's renderer: buffers made from now on know the device's precision when they are made */
export const setFeedbackRenderer = (renderer: Pick<FeedbackRenderer, "extensions"> | null): void => {
  probe = renderer;
};

let feedbacksHeld = false;

/**
 * Hold every buffer (the editor's Live switch off – `ArScene.holdClock`, #275): no step, each keeps its last frame and
 * its time, and goes on from there when the hold ends
 */
export const holdFeedbacks = (held: boolean): void => {
  feedbacksHeld = held;
};

/**
 * Step every live buffer once (creation order), before the scene renders – none while they are held; the render target is left unbound (null).
 * Called by the view's loop in each frame it draws.
 */
export const stepFeedbacks = (renderer: FeedbackRenderer, delta: number): void => {
  if (!buffers.size || feedbacksHeld) return;
  const previous = renderer.getRenderTarget();
  let stepped = false;
  buffers.forEach(buffer => {
    if (!buffer.live()) return;
    buffer.step(renderer, delta);
    stepped = true;
  });
  if (stepped) renderer.setRenderTarget(previous);
};

/**
 * The buffers made while `build` runs belong to the node built: `own` gives them its object, `dispose` frees them. A
 * post-processing pass's (made in its factory) have no object: `hold` stops them while the chain renders without passes.
 */
export interface OwnedFeedbacks {
  own(object: Object3D): void;
  hold(held: boolean): void;
  dispose(): void;
  readonly count: number;
}

/** Run a node's build and collect the buffers it makes (nested builds collect their own) */
export const collectFeedbacks = <T>(build: () => T): [T, OwnedFeedbacks] => {
  const outer = scope;
  const made: FeedbackBuffer[] = [];
  scope = made;
  const owned: OwnedFeedbacks = {
    own: object => made.forEach(buffer => (buffer.owner = object)),
    hold: held => made.forEach(buffer => (buffer.held = held)),
    dispose: () => made.forEach(buffer => buffer.dispose()),
    get count() { return made.length; },
  };
  try {
    return [build(), owned];
  } catch (error) {
    owned.dispose();
    throw error;
  } finally {
    scope = outer;
  }
};

/** A plugin uninstalled: its buffers no node owns go with it (a node's go with the node) */
export const disposePluginFeedbacks = (plugin: string): void => {
  [...buffers].forEach(buffer => { if (buffer.plugin === plugin && !buffer.owner) buffer.dispose(); });
};

/** How many buffers are alive (tests, the load test) */
export const feedbackCount = (): number => buffers.size;
