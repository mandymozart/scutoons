import {
  AmbientLight,
  AddEquation,
  Clock,
  CustomBlending,
  DirectionalLight,
  OneFactor,
  PerspectiveCamera,
  Scene,
  ShaderMaterial,
  Vector2,
  WebGLRenderer,
  ZeroFactor,
} from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { FullScreenQuad } from "three/examples/jsm/postprocessing/Pass.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { enabledPasses, type PassData } from "../types/index";
import { PostChain } from "./post/index";
import { setFeedbackRenderer, stepFeedbacks } from "./feedback";

/** Bloom: only what is brighter than white glows; rendered at half resolution */
const BLOOM_THRESHOLD = 1.0;
const BLOOM_RADIUS = 0.55;
const BLOOM_SCALE = 0.5;

/**
 * Adds the bloom over the camera image as light: colour is added, the canvas' alpha is left unchanged (a
 * premultiplied colour over alpha 0 is added to what lies under the canvas – the camera video).
 */
const bloomOverlayMaterial = () => new ShaderMaterial({
  uniforms: { tBloom: { value: null }, uStrength: { value: 0 } },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = vec4(position.xy, 0.0, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D tBloom;
    uniform float uStrength;
    varying vec2 vUv;
    void main() {
      gl_FragColor = vec4(texture2D(tBloom, vUv).rgb * uStrength, 0.0);
      #include <colorspace_fragment>
    }
  `,
  transparent: true,
  depthTest: false,
  depthWrite: false,
  blending: CustomBlending,
  blendEquation: AddEquation,
  blendSrc: OneFactor,
  blendDst: OneFactor,
  blendSrcAlpha: ZeroFactor,
  blendDstAlpha: OneFactor,
});

/**
 * The layer of objects that stand in for the camera picture (the editor's stage: table, surfaces, printed images, sky).
 * With passes, the image the passes shade holds them: bloom and vignette reach them as they reach the camera picture
 * in the reader, the grade stays on the AR layer (its mask). Without passes they render with everything else.
 */
export const BACKDROP_LAYER = 1;
const AR_LAYER = 0;

/**
 * Something drawn under the scene each frame (the look-around pass), while active: into the cleared screen, or into
 * the post chain's buffer while passes run – it draws into whatever render target is bound
 */
export interface Underlay {
  readonly active: boolean;
  render(): void;
}

/** How a frame is drawn (`drawPlan`) */
export interface DrawPlan {
  /** Through the post chain: passes are set and post-processing is on */
  chain: boolean;
  /**
   * Where the underlay goes: not drawn, onto the screen under the scene (no chain – as before passes existed, no cost),
   * or into the chain's buffer under the scene, so bloom, vignette and grade shade the world and its sky too
   */
  underlay: "none" | "screen" | "chain";
  /**
   * The chain's image holds more than the AR layer (the editor's desk, the underlay): a pass that reaches the AR layer
   * alone (the grade with `reach: ar`) is given its coverage as a mask
   */
  mask: boolean;
}

/** The frame's plan from what is there: passes running, the underlay active, a backdrop on the stage – pure, for the tests */
export const drawPlan = ({ passes, underlay, backdrop }: { passes: boolean; underlay: boolean; backdrop: boolean }): DrawPlan => ({
  chain: passes,
  underlay: !underlay ? "none" : passes ? "chain" : "screen",
  mask: passes && (underlay || backdrop),
});

/**
 * The three.js side of the AR scene: one WebGL renderer (one context for the session), scene, camera,
 * lights and the render loop. The transparent canvas (`#scene`, faded in by main.css while AR runs) lies
 * over the camera video.
 */
export class ArView {
  readonly scene = new Scene();
  readonly camera = new PerspectiveCamera();
  readonly renderer: WebGLRenderer;
  private clock = new Clock();
  private frameListeners = new Set<(delta: number) => void>();
  private looping = false;
  /** The canvas shows nothing (last frame was rendered empty) */
  private cleared = true;
  /**
   * Whether a frame needs drawing (something visible or animating). While nothing is found, rendering
   * costs no GPU time and the tracking engine has the GPU to itself (battery, heat).
   */
  needsRender: () => boolean = () => true;
  /** Bloom strength (0 = off – no post-processing cost); set every frame by the scene's animations */
  bloomStrength = 0;
  underlay: Underlay | null = null;
  /** The content's post-processing over the rendered scene – built only while a scene names passes */
  private post: PostChain | null = null;
  private postProcessing = true;
  /** Follows the container's size */
  private observer: ResizeObserver | null;
  private bloom: { composer: EffectComposer; pass: UnrealBloomPass; quad: FullScreenQuad; material: ShaderMaterial } | null = null;

  constructor(private container: HTMLElement, private onResize: (camera: PerspectiveCamera) => void) {
    this.renderer = new WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    // The plugins' feedback buffers learn the device's precision from it
    setFeedbackRenderer(this.renderer);
    const canvas = this.renderer.domElement;
    canvas.id = "scene";
    Object.assign(canvas.style, { position: "absolute", left: "0", top: "0" });
    container.appendChild(canvas);

    // The lighting the models were authored for: ambient #BBB, directional 0.6 from above left
    const ambient = new AmbientLight(0xbbbbbb, 1);
    const sun = new DirectionalLight(0xffffff, 0.6);
    sun.position.set(-0.5, 1, 1);
    this.scene.add(ambient, sun);
    // The lights reach the backdrop layer too; the camera sees both layers
    ambient.layers.enable(BACKDROP_LAYER);
    sun.layers.enable(BACKDROP_LAYER);
    this.camera.layers.enable(BACKDROP_LAYER);

    window.addEventListener("resize", this.resize);
    // The container can change size without the window (the editor's columns are dragged, #144)
    this.observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(() => this.resize());
    this.observer?.observe(container);
    this.resize();
  }

  /** Called every rendered frame with the seconds since the last one */
  /** The canvas – editor tools listen on it */
  get domElement(): HTMLCanvasElement {
    return this.renderer.domElement;
  }

  onFrame(listener: (delta: number) => void): () => void {
    this.frameListeners.add(listener);
    return () => this.frameListeners.delete(listener);
  }

  get running(): boolean {
    return this.looping;
  }

  start(): void {
    if (this.running) return;
    this.clock.getDelta(); // no jump after a pause
    this.looping = true;
    this.renderer.setAnimationLoop(() => {
      const delta = this.clock.getDelta();
      this.frameListeners.forEach(listener => listener(delta));
      const needed = this.needsRender();
      if (!needed && this.cleared) return;
      // The plugins' feedback buffers step once per drawn frame, before the scene that shows them
      if (needed) stepFeedbacks(this.renderer, delta);
      this.draw(); // the first frame with nothing visible clears the canvas
      this.cleared = !needed;
    });
  }

  /**
   * The colour under a point of the canvas (CSS px from its top left) as `#rrggbb`: the frame is drawn again and the
   * pixel read right after (the drawing buffer is not preserved between frames) – the editor's "pick from the stage"
   */
  sample(x: number, y: number): string | null {
    const gl = this.renderer.getContext();
    const size = this.renderer.getSize(new Vector2());
    const dpr = this.renderer.getPixelRatio();
    if (x < 0 || y < 0 || x > size.x || y > size.y) return null;
    this.draw();
    const pixel = new Uint8Array(4);
    gl.readPixels(Math.min(Math.round(x * dpr), Math.round(size.x * dpr) - 1), Math.min(Math.round((size.y - y) * dpr), Math.round(size.y * dpr) - 1), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    return `#${[pixel[0], pixel[1], pixel[2]].map(v => v.toString(16).padStart(2, "0")).join("")}`;
  }

  /**
   * The passes over the rendered scene (`passes` of the scene or the experience), in order; an empty list frees the chain
   * and the scene renders straight to the canvas again – as does a list whose passes are all off (#233)
   */
  setPasses(passes: readonly PassData[]): void {
    if (!enabledPasses(passes).length) {
      this.post?.dispose();
      this.post = null;
    } else {
      this.post ??= new PostChain(this.renderer, this.scene, this.camera);
      this.post.enabled = this.postProcessing;
      this.post.set(passes);
    }
    this.cleared = false;
  }

  /** Post-processing on or off (the passes stay set) */
  setPostProcessing(enabled: boolean): void {
    this.postProcessing = enabled;
    if (this.post) this.post.enabled = enabled;
    this.cleared = false;
  }

  /** Stop rendering – the canvas keeps the last frame */
  stop(): void {
    this.looping = false;
    this.renderer.setAnimationLoop(null);
  }

  /**
   * The underlay (the look-around world and sky) and the scene – through the content's passes while a scene or the
   * experience names them, the underlay then inside the chain – plus the bloom over it while an animation wants it
   */
  private draw(): void {
    const bloom = this.bloomStrength > 0 ? this.ensureBloom() : null;
    bloom?.composer.render(); // scene → half-size target → bloom (only the part brighter than white)
    const autoClear = this.renderer.autoClear;
    this.renderer.setRenderTarget(null);
    const post = this.post?.active ? this.post : null;
    const underlay = this.underlay?.active ? this.underlay : null;
    const plan = drawPlan({ passes: !!post, underlay: !!underlay, backdrop: !!post && this.hasBackdrop() });
    if (underlay && plan.underlay === "screen") {
      this.renderer.clear();
      underlay.render();
      this.renderer.setRenderTarget(null);
      this.renderer.clearDepth();
      this.renderer.autoClear = false;
    }
    if (post) {
      // The whole picture through the passes: the backdrop (the camera picture's stand-ins) and the underlay are part
      // of the image they shade, a grade with `reach: ar` is limited to the AR layer by its mask
      this.renderer.clear();
      this.renderer.autoClear = false;
      post.render(plan.mask ? { arLayers: 1 << AR_LAYER } : null, plan.underlay === "chain" ? this.drawUnderlay : null);
    } else {
      this.renderer.render(this.scene, this.camera);
    }
    if (bloom) {
      bloom.material.uniforms.tBloom.value = bloom.pass.renderTargetsHorizontal[0].texture;
      bloom.material.uniforms.uStrength.value = this.bloomStrength;
      this.renderer.autoClear = false;
      bloom.quad.render(this.renderer);
    }
    this.renderer.autoClear = autoClear;
  }

  /** The underlay into the bound target (the chain's buffer) – one function for every frame */
  private drawUnderlay = (): void => this.underlay?.render();

  /** Objects stand in for the camera picture (a light does not) */
  private hasBackdrop(): boolean {
    return this.scene.children.some(child => !(child as { isLight?: boolean }).isLight && child.layers.isEnabled(BACKDROP_LAYER) && !child.layers.isEnabled(AR_LAYER));
  }

  private ensureBloom() {
    if (!this.bloom) {
      const size = this.renderer.getSize(new Vector2());
      const composer = new EffectComposer(this.renderer);
      composer.renderToScreen = false;
      composer.setPixelRatio(this.renderer.getPixelRatio() * BLOOM_SCALE);
      composer.setSize(size.x, size.y);
      composer.addPass(new RenderPass(this.scene, this.camera));
      const pass = new UnrealBloomPass(new Vector2(size.x, size.y), 1, BLOOM_RADIUS, BLOOM_THRESHOLD);
      composer.addPass(pass);
      const material = bloomOverlayMaterial();
      this.bloom = { composer, pass, material, quad: new FullScreenQuad(material) };
    }
    return this.bloom;
  }

  resize = (): void => {
    const width = this.container.clientWidth;
    const height = this.container.clientHeight;
    if (!width || !height) return;
    this.renderer.setSize(width, height);
    this.bloom?.composer.setSize(width, height);
    this.post?.setSize(width, height);
    this.camera.aspect = width / height;
    this.onResize(this.camera); // the tracker fits video + field of view
    this.camera.updateProjectionMatrix();
    // A resized canvas is cleared by the browser: draw at once, also while the loop runs, or the next paint shows a
    // blank frame before the loop's next frame (the flicker while a panel is dragged)
    this.draw();
    this.cleared = false; // size changed: draw again
  };

  dispose(): void {
    this.stop();
    window.removeEventListener("resize", this.resize);
    this.observer?.disconnect();
    this.observer = null;
    this.frameListeners.clear();
    this.post?.dispose();
    this.post = null;
    if (this.bloom) {
      this.bloom.composer.dispose();
      this.bloom.pass.dispose();
      this.bloom.quad.dispose();
      this.bloom.material.dispose();
      this.bloom = null;
    }
    setFeedbackRenderer(null);
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }
}
