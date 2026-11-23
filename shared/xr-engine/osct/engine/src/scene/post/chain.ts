import { type Camera, Color, CustomBlending, HalfFloatType, OneFactor, OneMinusSrcAlphaFactor, type Scene, Vector2, type WebGLRenderer, WebGLRenderTarget } from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { Pass } from "three/examples/jsm/postprocessing/Pass.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass.js";
import { enabledPasses, passBudgetWarning, type PassData } from "../../types/index";
import { collectFeedbacks, type OwnedFeedbacks } from "../feedback";
import { type ResolvedTextures, resolveTextures, sameTextures } from "../textures";
import { ENGINE_OWNER, type MaskUsage, type PassContext, passContext, passRegistration, type PassRegistration, type PostPass } from "./registry";

/** Anti-aliasing of the chain's buffer – as the canvas itself has it (clamped to what the GPU offers) */
const SAMPLES = 4;

/**
 * The last pass: the chain's buffer (linear, premultiplied) to the canvas – the colour is divided by the alpha for
 * the transfer to the output colour space and multiplied again, so an edge looks as it does without a chain; light
 * over alpha 0 (a bloom over the camera picture) is converted as it is. Drawn over what the screen holds with
 * premultiplied blending, as the scene itself would be.
 */
const OUTPUT_SHADER = {
  uniforms: { tDiffuse: { value: null } },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    varying vec2 vUv;
    void main() {
      vec4 c = texture2D(tDiffuse, vUv);
      if (c.a > 0.001) {
        gl_FragColor = vec4(linearToOutputTexel(vec4(c.rgb / c.a, 1.0)).rgb * c.a, c.a);
      } else {
        gl_FragColor = vec4(linearToOutputTexel(vec4(c.rgb, 1.0)).rgb, c.a);
      }
    }
  `,
};

/**
 * The first pass while something is drawn under the scene (the look-around world and sky): the chain's buffer is
 * cleared and the underlay drawn into it, so the scene lies over it there and every pass shades both. The underlay
 * writes premultiplied colour over alpha like the scene; into the buffer its shader outputs linear (three picks the
 * output colour space by the render target), as the buffer holds.
 */
class UnderlayPass extends Pass {
  draw: (() => void) | null = null;

  constructor() {
    super();
    this.needsSwap = false;
    this.enabled = false;
  }

  render(renderer: WebGLRenderer, _writeBuffer: WebGLRenderTarget, readBuffer: WebGLRenderTarget): void {
    const autoClear = renderer.autoClear;
    renderer.setRenderTarget(readBuffer);
    renderer.clear();
    renderer.autoClear = false;
    this.draw?.();
    renderer.setRenderTarget(readBuffer);
    renderer.autoClear = autoClear;
  }
}

interface Instance {
  data: PassData;
  pass: PostPass;
  /** What built it: a pass of another registration (a plugin run again) is built anew */
  registration: PassRegistration;
  /** Its parameters as resolved for the factory or the last update (texture parameters as the content names them) */
  applied: Record<string, unknown>;
  /** Its texture parameters as three.js textures (a plugin's pass – parameters of kind `texture`) */
  textures: ResolvedTextures;
  /** The feedback buffers its factory made – they step while it runs, they go with it */
  feedbacks: OwnedFeedbacks | null;
}

/** The texture parameters of a resolved set as the textures a pass got (a live edit keeps them) */
const withTextures = (registration: PassRegistration, values: Record<string, unknown>, textures: ResolvedTextures): Record<string, unknown> => {
  const next = { ...values };
  Object.entries(registration.spec.params).forEach(([name, param]) => { if (param.kind === "texture") next[name] = textures.values[name]; });
  return next;
};

const describeError = (error: unknown): string => (error instanceof Error ? `${error.name}: ${error.message}` : String(error));

/**
 * The post-processing chain over the view's one renderer: the scene is rendered into a buffer, the passes of the
 * content run over it in order (`passes: [{ type, … }]`, from the pass registry), and the result lies on the canvas
 * as the scene would. Built only while a scene names passes; `set([])` frees everything – a scene without passes
 * renders straight to the canvas as before (no cost). `enabled` off keeps the passes but renders without them (the
 * editor's viewport outside *rendered*). A pass the content switched off (`enabled: false`, #233) is not built: the
 * chain runs the others as if it were not in the list.
 *
 * A plugin's pass is built from its factory in its place in the list, like a built-in one. A pass
 * that cannot be built – its factory throws or returns no three.js pass – is logged and skipped, the others run; one
 * whose `render` throws is logged and switched off for the chain's life (that frame keeps the image so far). Its texture
 * parameters arrive as textures; the feedback buffers its factory makes step only while the chain renders with passes.
 */
export class PostChain {
  private composer: EffectComposer;
  private renderPass: RenderPass;
  private underlayPass = new UnderlayPass();
  private output: ShaderPass;
  private instances: Instance[] = [];
  private size = new Vector2();
  /** The AR layer's coverage alone – what a pass that reaches only the AR layer (the grade with `reach: ar`) is masked with */
  private mask: WebGLRenderTarget;
  private on = true;
  /** The last phone budget warning (logged once per change, not on every live edit) */
  private budget: string | null = null;
  /** When the last frame rendered (procedural textures of a pass advance with it) */
  private last: number | null = null;

  constructor(private renderer: WebGLRenderer, private scene: Scene, private camera: Camera) {
    const size = renderer.getSize(new Vector2());
    const pixelRatio = renderer.getPixelRatio();
    const target = new WebGLRenderTarget(size.x * pixelRatio, size.y * pixelRatio, {
      type: HalfFloatType,
      samples: Math.min(SAMPLES, renderer.capabilities.maxSamples),
    });
    target.texture.name = "PostChain.rt1";
    this.mask = new WebGLRenderTarget(size.x * pixelRatio, size.y * pixelRatio, { samples: Math.min(SAMPLES, renderer.capabilities.maxSamples) });
    this.composer = new EffectComposer(renderer, target);
    this.composer.setPixelRatio(pixelRatio);
    this.composer.setSize(size.x, size.y);
    this.renderPass = new RenderPass(scene, camera);
    this.output = new ShaderPass(OUTPUT_SHADER);
    Object.assign(this.output.material, {
      transparent: true,
      depthTest: false,
      depthWrite: false,
      blending: CustomBlending,
      blendSrc: OneFactor,
      blendDst: OneMinusSrcAlphaFactor,
      blendSrcAlpha: OneFactor,
      blendDstAlpha: OneMinusSrcAlphaFactor,
    });
    this.size.copy(size);
    this.rebuild();
  }

  /** Off: the passes stay, the scene renders without them (and their feedback buffers wait) */
  get enabled(): boolean {
    return this.on;
  }

  set enabled(enabled: boolean) {
    this.on = enabled;
    this.instances.forEach(i => i.feedbacks?.hold(!enabled));
  }

  /** Something to run this frame */
  get active(): boolean {
    return this.enabled && this.instances.length > 0;
  }

  get passes(): readonly PassData[] {
    return this.instances.map(i => i.data);
  }

  /**
   * The passes to run, in order. A pass whose type, registration and position stay gets its new parameters in place
   * (`update` – unless a texture parameter names another texture); everything else is built anew. An unknown type is
   * logged and skipped (nothing registered it), and so is a pass whose factory fails. A pass that is off is left out.
   */
  set(list: readonly PassData[]): void {
    const passes = enabledPasses(list);
    const pixelRatio = this.renderer.getPixelRatio();
    const next: Instance[] = [];
    const kept = new Set<Instance>();
    passes.forEach((data, i) => {
      const registration = passRegistration(data.type);
      if (!registration) {
        console.warn(`[post] no pass "${data.type}" is registered – skipped`);
        return;
      }
      const context = passContext(registration, data, this.size, pixelRatio);
      const current = this.instances[i];
      if (current && current.data.type === data.type && current.registration === registration && current.pass.update
        && sameTextures(registration.spec.params, context.params, current.applied) && this.tune(current, context.params)) {
        kept.add(current);
        next.push({ ...current, data, applied: context.params });
        return;
      }
      const built = this.build(registration, data, context);
      if (built) next.push(built);
    });
    this.instances.forEach(instance => kept.has(instance) || this.free(instance));
    this.instances = next;
    this.rebuild();
    const budget = passBudgetWarning(passes);
    if (budget && budget !== this.budget) console.warn(`[post] ${budget}`);
    this.budget = budget;
  }

  /** New values into a pass in place; false (logged) when its `update` throws – it is built anew then */
  private tune(instance: Instance, params: Record<string, unknown>): boolean {
    try {
      instance.pass.update!(withTextures(instance.registration, params, instance.textures));
      return true;
    } catch (error) {
      console.error(`[post] the pass "${instance.data.type}" (${instance.registration.owner}) failed to update – built anew:`, error);
      return false;
    }
  }

  /** A pass from its factory with its textures and feedback buffers; null (logged) when it cannot be built */
  private build(registration: PassRegistration, data: PassData, context: PassContext): Instance | null {
    const textures = resolveTextures(registration.spec.params, context.params, `pass ${data.type}`);
    let pass: PostPass;
    let feedbacks: OwnedFeedbacks;
    try {
      [pass, feedbacks] = collectFeedbacks(() => registration.factory({ ...context, params: textures.values }));
    } catch (error) {
      console.error(`[post] the pass "${data.type}" (${registration.owner}) could not be built – skipped:`, error);
      textures.dispose();
      return null;
    }
    if (!pass || typeof pass.render !== "function") {
      console.error(`[post] the pass "${data.type}" (${registration.owner}) returned no three.js pass – skipped`);
      feedbacks.dispose();
      textures.dispose();
      return null;
    }
    // A plugin's pass may be an object with `render` alone (three's Pass has the rest)
    pass.setSize ??= () => undefined;
    pass.dispose ??= () => undefined;
    if (registration.owner !== ENGINE_OWNER) this.guard(pass, data.type, registration.owner);
    feedbacks.hold(!this.on);
    return { data, pass, registration, applied: context.params, textures, feedbacks: feedbacks.count ? feedbacks : null };
  }

  /**
   * A plugin's pass that throws while it renders is logged and switched off: that frame keeps the image so far (no swap
   * of the buffer it did not write), the rest of the chain runs, and every later frame skips it
   */
  private guard(pass: PostPass, type: string, owner: string): void {
    const render = pass.render.bind(pass);
    const swaps = pass.needsSwap;
    pass.render = (...args: Parameters<PostPass["render"]>) => {
      pass.needsSwap = swaps;
      try {
        render(...args);
      } catch (error) {
        console.error(`[post] the pass "${type}" (${owner}) failed while rendering – switched off: ${describeError(error)}`);
        pass.enabled = false;
        pass.needsSwap = false;
      }
    };
  }

  private free(instance: Instance): void {
    try {
      instance.pass.dispose();
    } catch (error) {
      console.error(`[post] the pass "${instance.data.type}" (${instance.registration.owner}) failed to dispose:`, error);
    }
    instance.feedbacks?.dispose();
    instance.textures.dispose();
  }

  private rebuild(): void {
    this.composer.passes = [this.underlayPass, this.renderPass, ...this.instances.map(i => i.pass), this.output];
    const pixelRatio = this.renderer.getPixelRatio();
    this.instances.forEach(i => this.resize(i, this.size.x * pixelRatio, this.size.y * pixelRatio));
  }

  private resize(instance: Instance, width: number, height: number): void {
    try {
      instance.pass.setSize(width, height);
    } catch (error) {
      console.error(`[post] the pass "${instance.data.type}" (${instance.registration.owner}) failed to resize:`, error);
    }
  }

  /** The canvas size in CSS px changed */
  setSize(width: number, height: number): void {
    this.size.set(width, height);
    this.composer.setPixelRatio(this.renderer.getPixelRatio());
    this.composer.setSize(width, height);
    const pixelRatio = this.renderer.getPixelRatio();
    this.mask.setSize(width * pixelRatio, height * pixelRatio);
  }

  /**
   * Render the scene through the chain onto the canvas (the caller cleared the screen).
   * `backdrop`: the image holds more than the AR layer – objects standing in for the camera picture (the editor's
   * desk) or the underlay – and `arLayers` is the camera's layer mask for the AR layer alone: a pass that reaches the
   * AR layer only is given its coverage as a mask (drawn only while such a pass uses it).
   * `underlay`: draws the look-around world and sky into the chain's buffer before the scene, so the passes shade it
   * too; `null`: the buffer starts empty and the screen keeps the camera picture under it.
   */
  render(backdrop: { arLayers: number } | null = null, underlay: (() => void) | null = null): void {
    const masked = !!backdrop && this.wantsMask();
    if (backdrop && masked) this.drawMask(backdrop.arLayers);
    const mask = masked ? this.mask.texture : null;
    this.instances.forEach(i => i.pass.setMask?.(mask));
    this.underlayPass.draw = underlay;
    this.underlayPass.enabled = !!underlay;
    this.renderPass.clear = !underlay;
    // Procedural textures of the passes advance with the frames; the passes get the time since the last one
    const now = performance.now();
    const delta = this.last === null ? 0 : Math.min((now - this.last) / 1000, 0.1);
    this.last = now;
    this.instances.forEach(i => i.textures.update(delta));
    this.composer.render(delta);
    this.underlayPass.draw = null;
  }

  /** A pass limits itself to the AR layer with its current parameters (a pass with `setMask` and no `usesMask` always does) */
  private wantsMask(): boolean {
    return this.instances.some(i => !!i.pass.setMask && (i.pass as PostPass & MaskUsage).usesMask !== false);
  }

  /** The AR layer alone into the mask target: its alpha is the coverage (the scene's background stays out) */
  private drawMask(arLayers: number): void {
    const { renderer, scene, camera } = this;
    const layers = camera.layers.mask;
    const background = scene.background;
    const color = renderer.getClearColor(new Color());
    const alpha = renderer.getClearAlpha();
    camera.layers.mask = arLayers;
    scene.background = null;
    renderer.setRenderTarget(this.mask);
    renderer.setClearColor(0x000000, 0);
    renderer.clear();
    renderer.render(scene, camera);
    renderer.setRenderTarget(null);
    renderer.setClearColor(color, alpha);
    scene.background = background;
    camera.layers.mask = layers;
  }

  dispose(): void {
    this.instances.forEach(instance => this.free(instance));
    this.instances = [];
    this.renderPass.dispose();
    this.underlayPass.dispose();
    this.mask.dispose();
    this.output.dispose();
    this.composer.dispose();
  }
}
