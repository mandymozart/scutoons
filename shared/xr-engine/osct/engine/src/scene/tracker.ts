import type { PerspectiveCamera } from "three";
import { ArSceneError, ErrorCode, type Target } from "../types/index";
import { getTargetBudget } from "../utils/prepared-scenes";
import { targetRecognition } from "../utils/recognition-setting";
import { publishEngineState, EMPTY_ENGINE_STATE, registerLoadTest } from "../utils/tracker-debug";
import { setCameraTrack } from "../utils/camera-debug";
import { applyCameraDefaults, releaseCameraRequest, wrapCameraRequest } from "../utils/camera-request";
import { runLoadTest } from "./load-test";
import { CAMERA_START_TIMEOUT_MS, type IImageTracker, type ImageTrackerOptions, type TrackedScene } from "./tracker-types";
import {
  anchorMatrix,
  loadXr8,
  makeImageTarget,
  preparedImageTarget,
  type PreparedImageTarget,
  type XR8Tracking,
  type XrImageDetail,
  type XrPipelineModule,
  type XrReality,
} from "./xr8";

/** The engine's own shader for the camera picture (8th Wall `GlTextureRenderer`'s default) – restored when no grade reaches it */
const CAMERA_SHADER = "precision lowp float;\nvarying vec2 texUv;\nuniform sampler2D sampler;\nvoid main() {\n  gl_FragColor = texture2D(sampler, texUv);\n}\n";
/** Loading the targets into the engine may take this long before tracking starts anyway */
const TARGETS_LOAD_TIMEOUT_MS = 15000;
/** A neighbouring scene's target seen this long (and none of the current scene's) switches the scene */
const SCENE_SEEN_MS = 400;
/** `whenSettled()` gives up after this long (a window of the whole experience on a slow phone) */
const SETTLE_TIMEOUT_MS = 180000;
/**
 * Grow mode: targets added per batch. Each new target costs ~0.35 s of main-thread work (preparing ~0.16 s,
 * extracting ~0.19 s on a mid-range phone), so a batch stays short enough not to hold up a page switch.
 */
const GROW_BATCH_TARGETS = 3;
/** Grow mode: pause between batches, and the longest wait for idle time */
const GROW_PAUSE_MS = 150;
const GROW_IDLE_TIMEOUT_MS = 1000;

const idle = () => new Promise<void>(resolve => {
  if (typeof window.requestIdleCallback === "function") window.requestIdleCallback(() => resolve(), { timeout: GROW_IDLE_TIMEOUT_MS });
  else window.setTimeout(resolve, 50);
});
const NEAR = 0.01;
const FAR = 1000;

interface SceneTargets {
  sceneId: string;
  targets: PreparedImageTarget[];
}

interface LoadedTarget {
  id: string;
  /** Anchor scale per engine scale: scaled width × full image width ÷ tracked width */
  widthFactor: number;
}

/**
 * Camera + image tracking with the 8th Wall engine (`@8thwall/engine`, MIT). The camera survives a scene switch, `loadTargets()` swaps the tracked image targets (made from the target images, `xr8.ts`).
 *
 * Scene switches: the engine extracts each target's features on the device (one target per frame – the engine takes
 * no precompiled features; a target's `recognition` made ahead saves only the app's part, the grey frame, #306). So the neighbouring scenes' targets stay loaded next to the current ones (`prepareTargets()`)
 * – the app is told only about the current scene's, and a switch to a neighbour starts at once. The engine keeps what it already has when it
 * is configured again and extracts only new targets. A neighbour's target seen steadily while none of the
 * current scene's is means the reader turned the page: `onSceneSeen` (the app switches the scene).
 *
 * The engine owns the camera: it draws the picture into its own canvas under the three.js canvas (a second
 * WebGL context) and reports poses in its scene. World tracking is off, so the
 * camera stays put; each target's anchor is its pose relative to the camera – the three.js camera stays at
 * the origin and gets the engine's projection, as in the engine's own three.js module.
 */
export class ImageTracker implements IImageTracker {
  private XR8: XR8Tracking | null = null;
  private canvas: HTMLCanvasElement | null = null;
  /** The camera picture's shader (a grade with `reach: all`), null = the engine's own */
  private cameraShader: string | null = null;
  private camera: PerspectiveCamera | null = null;
  private intrinsics: number[] | null = null;
  private started = false;
  private paused = false;
  /** Engine target name → app target, for the scene being tracked (empty while none is) */
  private targets = new Map<string, LoadedTarget>();
  /** Found targets' latest engine poses, by target id */
  private poses = new Map<string, { detail: XrImageDetail; target: LoadedTarget }>();
  private loadRun = 0;
  private imageTargets = new Map<string, Promise<PreparedImageTarget>>();
  /** The current scene's targets and the neighbours' (`prepareTargets()`) – what the engine keeps loaded */
  private current: SceneTargets | null = null;
  private upcoming: SceneTargets[] = [];
  /** Engine target name → its scene, for the upcoming scenes' targets */
  private sceneOf = new Map<string, string>();
  /** Upcoming scenes' targets in view, since when (performance.now()) */
  private seenSince = new Map<string, number>();
  /** When the current scene's targets were ready (for the found log of the load test) */
  private currentSince = 0;
  /** Engine target names whose features are extracted (reported by its `imagescanning` event) */
  private ready = new Set<string>();
  private prepareRun = 0;
  private startWaiter: { resolve: () => void; reject: (error: Error) => void } | null = null;
  private scanningWaiter: (() => void) | null = null;
  /** Engine target name → app target id, for every target ever prepared (debug view of what the engine sees) */
  private idOf = new Map<string, string>();
  /** Targets the engine reports as found right now, any loaded scene (name → target id) – debug view */
  private seen = new Map<string, string>();
  /** The last automatic scene switch the tracker asked for – debug view */
  private lastSwitch: { sceneId: string; targetId: string; at: number } | null = null;
  /** Grow mode: scenes still to add, nearest first, and whether the growth loop runs */
  private growQueue: TrackedScene[] = [];
  private growing = false;
  /** Grow mode is on (`growTargets()`), until a fixed window (`prepareTargets()`) replaces it */
  private growMode = false;
  /** Resolved once nothing is being prepared or extracted (`whenSettled()`) */
  private settleWaiters = new Set<() => void>();
  /** The latest window (`prepareTargets()`): when it was configured, and how long preparing / extracting took */
  private windowTimes = { configuredAt: 0, prepareMs: 0, extractMs: 0 };
  /** Scenes whose target images are being prepared (debug view) */
  private preparing: string[] = [];
  /** The engine is extracting since (performance.now()), null when idle (debug view) */
  private extractingSince: number | null = null;
  /** Counters for the load test: engine frames, frames with a current target found, current targets lost */
  private counters = { frames: 0, framesFound: 0, lost: 0 };

  /** `xr8Base`: where the app serves the 8th Wall engine's files (`xr.js`, `xr-tracking.js`), ending in `/` */
  constructor(private container: HTMLElement, private options: ImageTrackerOptions, private xr8Base: string) {}

  get hasCamera(): boolean {
    return this.started;
  }

  get tracking(): boolean {
    return this.targets.size > 0;
  }

  /** The engine plays the stream in a hidden video next to its canvas */
  get cameraVideo(): HTMLVideoElement | null {
    if (!this.started) return null;
    const videos = Array.from(this.container.querySelectorAll("video"));
    return videos.find(video => video.srcObject instanceof MediaStream) ?? null;
  }

  /** The scene being tracked (load test) */
  get currentSceneId(): string | null {
    return this.current?.sceneId ?? null;
  }

  /** Targets of the current scene found right now (load test) */
  get foundCount(): number {
    return this.poses.size;
  }

  get isPaused(): boolean {
    return this.paused;
  }

  /** Targets and scenes the engine holds: the current scene + the window (load test) */
  get heldTargets(): number {
    return (this.current?.targets.length ?? 0) + this.upcoming.reduce((n, scene) => n + scene.targets.length, 0);
  }

  get heldScenes(): number {
    return (this.current ? 1 : 0) + this.upcoming.length;
  }

  /** Load test counters since the camera started: engine frames, frames with a find, current targets lost */
  get stats(): { frames: number; framesFound: number; lost: number } {
    return { ...this.counters };
  }

  async startCamera(): Promise<void> {
    if (this.started) return;
    const XR8 = await loadXr8(this.xr8Base);
    this.XR8 = XR8;

    const canvas = document.createElement("canvas");
    canvas.className = "ar-camera";
    Object.assign(canvas.style, { position: "absolute", top: "0", left: "0", width: "100%", height: "100%", zIndex: "-2" });
    this.container.appendChild(canvas);
    this.canvas = canvas;
    this.sizeCanvas();

    const started = new Promise<void>((resolve, reject) => {
      const timer = window.setTimeout(
        () => reject(new ArSceneError(ErrorCode.CAMERA_NOT_RESPONDING, `camera not responding (no picture after ${CAMERA_START_TIMEOUT_MS / 1000} s)`)),
        CAMERA_START_TIMEOUT_MS,
      );
      this.startWaiter = {
        resolve: () => { window.clearTimeout(timer); resolve(); },
        reject: error => { window.clearTimeout(timer); reject(error); },
      };
    });
    XR8.XrController.configure({ disableWorldTracking: true, imageTargetData: [] });
    XR8.addCameraPipelineModules([XR8.GlTextureRenderer.pipelineModule(), XR8.XrController.pipelineModule(), this.module()]);
    if (this.cameraShader) XR8.GlTextureRenderer.configure({ fragmentSource: this.cameraShader });
    // The engine's camera request gets the main back lens (and the debug zoom request) – camera-request.ts
    wrapCameraRequest();
    // Any device: desktop browsers (dev, tests) too – the default allows phones only
    XR8.run({ canvas, allowedDevices: XR8.XrConfig.device().ANY, cameraConfig: { direction: XR8.XrConfig.camera().BACK } });
    try {
      await started;
    } catch (error) {
      this.stop();
      throw error;
    } finally {
      this.startWaiter = null;
    }
    this.started = true;
    registerLoadTest((options, onProgress) => runLoadTest(this, options, onProgress));
    this.publishState();
  }

  async loadTargets({ sceneId, targets }: TrackedScene): Promise<void> {
    const XR8 = this.XR8;
    if (!XR8 || !this.started) throw new Error("loadTargets() needs the camera");
    this.stopTracking();
    const run = this.loadRun;

    const begun = performance.now();
    const withImage = targets.filter(target => target.imageSrc);
    const data = await Promise.all(withImage.map(target => this.imageTarget(target)));
    if (run !== this.loadRun) return; // replaced or stopped meanwhile
    this.targets = new Map(data.map((target, i) => [
      target.data.name,
      { id: withImage[i].id, widthFactor: target.widthFactor },
    ]));
    data.forEach((target, i) => this.idOf.set(target.data.name, withImage[i].id));
    // The previous scene stays loaded until prepareTargets() names the new neighbours (it is one of them)
    const previous = this.current && this.current.sceneId !== sceneId ? [this.current] : [];
    this.upcoming = [...this.upcoming.filter(scene => scene.sceneId !== sceneId), ...previous];
    this.current = { sceneId, targets: data };
    this.configureEngine();
    this.currentSince = performance.now();
    if (!data.length) return;
    if (data.every(target => this.ready.has(target.data.name))) {
      console.info(`[8th Wall] ${data.length} targets already loaded – ready after ${Math.round(performance.now() - begun)} ms`);
      return;
    }

    let waiter: () => void = () => {};
    const scanning = new Promise<void>(resolve => {
      const timer = window.setTimeout(() => {
        console.warn("[8th Wall] Targets still loading – tracking starts anyway");
        resolve();
      }, TARGETS_LOAD_TIMEOUT_MS);
      waiter = () => { window.clearTimeout(timer); resolve(); };
    });
    this.scanningWaiter = waiter;
    const configured = performance.now();
    await scanning;
    this.currentSince = performance.now();
    console.info(`[8th Wall] ${data.length} targets: prepared in ${Math.round(configured - begun)} ms, engine ready after ${Math.round(performance.now() - configured)} ms`);
    if (this.scanningWaiter === waiter) this.scanningWaiter = null;
  }

  prepareTargets(scenes: readonly TrackedScene[]): void {
    const run = ++this.prepareRun;
    this.growQueue = []; // a fixed window ends the growth
    this.growMode = false;
    const requested = performance.now();
    this.preparing = scenes.map(scene => scene.sceneId);
    this.publishState();
    void Promise.all(scenes.map(scene => this.prepareScene(scene))).then(
      upcoming => {
        if (run !== this.prepareRun || !this.started) return; // newer neighbours or the camera is off
        this.preparing = [];
        this.upcoming = upcoming;
        const now = performance.now();
        this.windowTimes = { configuredAt: now, prepareMs: now - requested, extractMs: 0 };
        this.configureEngine();
        console.info(`[8th Wall] ${this.ready.size} of ${(this.current?.targets.length ?? 0) + upcoming.reduce((n, scene) => n + scene.targets.length, 0)} targets extracted so far – next scenes loaded: ${upcoming.map(scene => `${scene.sceneId} (${scene.targets.length})`).join(", ")}`);
      },
      error => {
        if (run === this.prepareRun) this.preparing = [];
        this.publishState();
        console.warn("[8th Wall] Could not prepare the next scenes' targets:", error);
      },
    );
  }

  private prepareScene(scene: TrackedScene): Promise<SceneTargets> {
    const withImage = scene.targets.filter(target => target.imageSrc);
    return Promise.all(withImage.map(target => this.imageTarget(target))).then(targets => {
      targets.forEach((target, i) => this.idOf.set(target.data.name, withImage[i].id));
      return { sceneId: scene.sceneId, targets };
    });
  }

  growTargets(scenes: readonly TrackedScene[]): void {
    this.prepareRun++; // a running fixed-window prepare must not replace what grows
    if (!this.growing) this.preparing = []; // …and what it was preparing no longer counts
    this.growMode = true;
    this.growQueue = scenes.filter(scene => !this.holds(scene.sceneId));
    this.publishState();
    if (!this.growing) void this.grow();
  }

  private holds(sceneId: string): boolean {
    return this.current?.sceneId === sceneId || this.upcoming.some(scene => scene.sceneId === sceneId);
  }

  /**
   * The growth loop: waits until the engine has nothing to do and the page is idle, prepares the next few targets
   * (whole scenes, nearest first), adds them to what the engine holds, waits for their extraction, repeats until
   * the queue is empty. The engine restarts its queue with every configure, so batches never overlap.
   */
  private async grow(): Promise<void> {
    this.growing = true;
    try {
      while (this.started && this.growQueue.length) {
        try {
          await this.whenSettled();
        } catch {
          // The engine took very long: add the next batch anyway
        }
        await new Promise(resolve => window.setTimeout(resolve, GROW_PAUSE_MS));
        await idle();
        if (!this.started) break;
        const batch: TrackedScene[] = [];
        let count = 0;
        while (this.growQueue.length && (!batch.length || count + this.growQueue[0].targets.length <= GROW_BATCH_TARGETS)) {
          const scene = this.growQueue.shift()!;
          if (this.holds(scene.sceneId)) continue;
          batch.push(scene);
          count += scene.targets.length;
        }
        if (!batch.length) continue;
        this.preparing = batch.map(scene => scene.sceneId);
        this.publishState();
        const prepared = await Promise.all(batch.map(scene => this.prepareScene(scene)));
        this.preparing = [];
        // Stopped, or a fixed window took over meanwhile
        if (!this.started || !this.growMode) break;
        this.upcoming = [...this.upcoming, ...prepared.filter(scene => !this.holds(scene.sceneId))];
        this.windowTimes = { configuredAt: performance.now(), prepareMs: 0, extractMs: 0 };
        this.configureEngine();
      }
    } catch (error) {
      console.warn("[8th Wall] Growing the held targets stopped:", error);
    } finally {
      this.growing = false;
      this.publishState();
    }
  }

  /**
   * Resolves once the window is ready: nothing being prepared, nothing left to extract. With the latest window's
   * times – the app preparing its new target images, the engine extracting them (load test).
   */
  whenSettled(): Promise<{ prepareMs: number; extractMs: number }> {
    const times = () => ({ prepareMs: this.windowTimes.prepareMs, extractMs: this.windowTimes.extractMs });
    if (this.settled) return Promise.resolve(times());
    return new Promise((resolve, reject) => {
      const timer = window.setTimeout(() => {
        this.settleWaiters.delete(done);
        reject(new Error(`the targets were not ready within ${SETTLE_TIMEOUT_MS / 1000} s`));
      }, SETTLE_TIMEOUT_MS);
      const done = () => { window.clearTimeout(timer); resolve(times()); };
      this.settleWaiters.add(done);
    });
  }

  private get settled(): boolean {
    return !this.preparing.length && this.extractingSince === null;
  }

  /**
   * The engine's targets = current + upcoming. It unloads what is no longer listed, keeps the rest and
   * extracts the new ones (each new configure restarts that queue).
   */
  private configureEngine(): void {
    const XR8 = this.XR8;
    if (!XR8 || !this.started) return;
    const upcoming = this.upcoming;
    const scenes = this.current ? [this.current, ...upcoming] : upcoming;
    const byName = new Map(scenes.flatMap(scene => scene.targets).map(target => [target.data.name, target.data]));
    this.sceneOf = new Map(upcoming.flatMap(scene => scene.targets.map(target => [target.data.name, scene.sceneId])));
    this.ready.forEach(name => { if (!byName.has(name)) this.ready.delete(name); });
    // New targets: the engine extracts them now (the queue restarts with every configure)
    const extracting = [...byName.keys()].some(name => !this.ready.has(name));
    this.extractingSince = extracting ? (this.extractingSince ?? performance.now()) : null;
    XR8.XrController.configure({ imageTargetData: [...byName.values()] });
    this.publishState();
  }

  /** What the engine holds, for the debug overlay (`tracker-debug.ts`) */
  private publishState(): void {
    if (!this.started) {
      publishEngineState(EMPTY_ENGINE_STATE);
      return;
    }
    if (this.settled && this.settleWaiters.size) {
      const waiters = [...this.settleWaiters];
      this.settleWaiters.clear();
      waiters.forEach(waiter => waiter());
    }
    const upcoming = this.upcoming;
    const all = this.current ? [this.current, ...upcoming] : upcoming;
    const names = new Set(all.flatMap(scene => scene.targets.map(target => target.data.name)));
    publishEngineState({
      current: this.current ? { sceneId: this.current.sceneId, targets: this.current.targets.length } : null,
      held: upcoming.map(scene => ({ sceneId: scene.sceneId, targets: scene.targets.length })),
      preparing: [...this.preparing],
      targets: names.size,
      ready: [...names].filter(name => this.ready.has(name)).length,
      extractingSince: this.extractingSince,
      budget: getTargetBudget(),
      growing: this.growing || this.growQueue.length > 0 ? this.growQueue.length : null,
      seen: [...this.seen.values()],
      lastSwitch: this.lastSwitch,
    });
  }

  pause(): void {
    if (!this.XR8 || !this.started || this.paused) return;
    this.XR8.pause();
    this.paused = true;
    this.poses.clear();
    this.seen.clear();
    this.publishState();
  }

  resume(): void {
    if (!this.XR8 || !this.paused) return;
    this.XR8.resume();
    this.paused = false;
  }

  stopTracking(): void {
    this.loadRun++;
    this.targets.clear();
    this.poses.clear();
    this.scanningWaiter?.();
    this.scanningWaiter = null;
    this.seenSince.clear();
    // The engine keeps its targets (a switch back or to a neighbour needs no new extraction)
  }

  gradeCamera(fragmentShader: string | null): void {
    if (fragmentShader === this.cameraShader) return;
    this.cameraShader = fragmentShader;
    // The engine keeps a shader it was given: none = its own again (`configure` ignores an empty source)
    if (this.XR8 && this.canvas) this.XR8.GlTextureRenderer.configure({ fragmentSource: fragmentShader ?? CAMERA_SHADER });
  }

  stop(): void {
    this.stopTracking();
    setCameraTrack(null);
    releaseCameraRequest();
    const XR8 = this.XR8;
    if (XR8 && this.canvas) {
      try {
        XR8.stop();
        XR8.clearCameraPipelineModules();
      } catch (error) {
        console.warn("[8th Wall] Stopping the engine failed:", error);
      }
    }
    this.canvas?.remove();
    this.canvas = null;
    this.current = null;
    this.upcoming = [];
    this.preparing = [];
    this.extractingSince = null;
    this.growQueue = [];
    this.growMode = false;
    this.seen.clear();
    // Waiters (the growth loop, the load test) go on and see that the camera is off
    const waiters = [...this.settleWaiters];
    this.settleWaiters.clear();
    waiters.forEach(waiter => waiter());
    this.sceneOf.clear();
    this.ready.clear();
    this.prepareRun++;
    this.started = false;
    this.paused = false;
    this.intrinsics = null;
    registerLoadTest(null);
    this.publishState();
  }

  fit(camera: PerspectiveCamera): void {
    this.camera = camera;
    this.sizeCanvas();
    this.applyProjection();
  }

  /**
   * Target data per image, once per session (the object URLs stay valid): the recognition data made ahead when the target
   * has it and the recognition path in force uses it (#306: the experience's `recognition: "prepared"`, or this device's
   * debug override, and no `recognitionOff` – `utils/recognition-setting.ts`; the engine loads its grey frame, nothing to
   * make here), else made from the target image now – the default path
   */
  private imageTarget(target: Pick<Target, "imageSrc" | "recognition" | "recognitionOff">): Promise<PreparedImageTarget> {
    const recognition = targetRecognition(target);
    if (recognition) return Promise.resolve(preparedImageTarget(recognition));
    const { imageSrc } = target;
    let data = this.imageTargets.get(imageSrc);
    if (!data) {
      data = makeImageTarget(imageSrc);
      data.catch(() => this.imageTargets.delete(imageSrc));
      this.imageTargets.set(imageSrc, data);
    }
    return data;
  }

  /** The engine draws the camera picture at the canvas' pixel size (cover); intrinsics follow its aspect */
  private sizeCanvas(): void {
    const canvas = this.canvas;
    if (!canvas) return;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    const width = Math.round(this.container.clientWidth * ratio);
    const height = Math.round(this.container.clientHeight * ratio);
    if (!width || !height || (canvas.width === width && canvas.height === height)) return;
    canvas.width = width;
    canvas.height = height;
  }

  /**
   * The engine's projection on the three.js camera, as fov / aspect / near / far – `ArView.resize()` calls
   * `updateProjectionMatrix()` after `fit()`, so a matrix set directly would be overwritten. The engine's
   * principal point is the centre (no offset to carry over).
   */
  private applyProjection(): void {
    const { camera, intrinsics: m } = this;
    if (!camera || !m || !m[0] || !m[5]) return;
    camera.fov = (2 * Math.atan(1 / m[5]) * 180) / Math.PI;
    camera.aspect = m[5] / m[0];
    const near = m[14] / (m[10] - 1);
    const far = m[14] / (m[10] + 1);
    camera.near = Number.isFinite(near) && near > 0 ? near : NEAR;
    camera.far = Number.isFinite(far) && far > camera.near ? far : FAR;
    camera.updateProjectionMatrix();
  }

  private setProjectionSize(width: number, height: number): void {
    this.XR8?.XrController?.updateCameraProjectionMatrix({
      cam: { pixelRectWidth: width, pixelRectHeight: height, nearClipPlane: NEAR, farClipPlane: FAR },
    });
  }

  /** Every frame: anchors of the found targets relative to the camera of that frame */
  private onFrame(reality: XrReality): void {
    const m = reality.intrinsics;
    if (m && m.every(Number.isFinite) && (!this.intrinsics || m.some((v, i) => v !== this.intrinsics![i]))) {
      this.intrinsics = [...m];
      this.applyProjection();
    }
    this.counters.frames++;
    if (this.poses.size) this.counters.framesFound++;
    this.poses.forEach(({ detail, target }, id) =>
      this.options.onUpdate(id, anchorMatrix(reality, detail, target.widthFactor)));
  }

  private onImage(detail: XrImageDetail, found: boolean): void {
    if (this.paused) return;
    this.noteSeen(detail.name, found);
    const target = this.targets.get(detail.name);
    if (!target) return this.onUpcomingImage(detail.name, found);
    const sceneId = this.current?.sceneId;
    if (found) {
      if (!this.poses.has(target.id)) {
        console.info(`[8th Wall] Found ${sceneId}/${target.id} ${Math.round(performance.now() - this.currentSince)} ms after the scene was ready (${this.poses.size + 1} found)`);
      }
      this.poses.set(target.id, { detail, target });
      return;
    }
    if (this.poses.delete(target.id)) {
      this.counters.lost++;
      console.info(`[8th Wall] Lost ${sceneId}/${target.id}`);
      this.options.onUpdate(target.id, null);
    }
  }

  /**
   * An upcoming scene's target: held in view (and nothing of the current scene) → report its scene; held on
   * (the app did not switch, e.g. not in scan mode), it is reported again after the same time
   */
  private onUpcomingImage(name: string, found: boolean): void {
    const sceneId = this.sceneOf.get(name);
    if (!sceneId) return;
    if (!found) {
      this.seenSince.delete(name);
      return;
    }
    const now = performance.now();
    const since = this.seenSince.get(name);
    if (since === undefined) {
      this.seenSince.set(name, now);
      console.info(`[8th Wall] Target of ${sceneId} in view (current scene: ${this.poses.size} found)`);
      return;
    }
    if (now - since < SCENE_SEEN_MS || this.poses.size > 0) return;
    this.seenSince.set(name, now);
    console.info(`[8th Wall] ${sceneId} held in view – switching`);
    this.lastSwitch = { sceneId, targetId: this.idOf.get(name) ?? name, at: Date.now() };
    this.publishState();
    this.options.onSceneSeen?.(sceneId);
  }

  /** Debug view: every target the engine finds or loses, whatever scene it belongs to */
  private noteSeen(name: string, found: boolean): void {
    if (found === this.seen.has(name)) return;
    if (found) this.seen.set(name, this.idOf.get(name) ?? name);
    else this.seen.delete(name);
    this.publishState();
  }

  private module(): XrPipelineModule {
    return {
      name: "osct-image-tracker",
      onStart: ({ canvasWidth, canvasHeight }) => {
        this.setProjectionSize(canvasWidth, canvasHeight);
        this.startWaiter?.resolve();
      },
      onCanvasSizeChange: ({ canvasWidth, canvasHeight }) => this.setProjectionSize(canvasWidth, canvasHeight),
      onCameraStatusChange: ({ status, reason, stream }) => {
        // The camera's defaults (continuous focus) and its track for the debug readout (camera-request.ts, camera-debug.ts)
        const track = status === "hasStream" ? stream?.getVideoTracks()[0] : undefined;
        if (track) void applyCameraDefaults(track).then(() => setCameraTrack(track));
        if (status === "failed") this.startWaiter?.reject(new Error(`Camera unavailable: ${reason ?? "failed"}`));
      },
      onException: error => {
        console.error("[8th Wall]", error);
        this.startWaiter?.reject(error instanceof Error ? error : new Error(String(error)));
      },
      onUpdate: ({ processCpuResult }) => {
        if (processCpuResult.reality) this.onFrame(processCpuResult.reality);
      },
      listeners: [
        {
          event: "reality.imagescanning",
          process: ({ detail }) => {
            (detail?.imageTargets ?? []).forEach((target: { name: string }) => this.ready.add(target.name));
            if (this.extractingSince !== null && this.windowTimes.configuredAt) {
              this.windowTimes.extractMs = performance.now() - Math.max(this.windowTimes.configuredAt, this.extractingSince);
            }
            this.extractingSince = null;
            this.scanningWaiter?.();
            this.publishState();
          },
        },
        { event: "reality.imagefound", process: ({ detail }) => this.onImage(detail, true) },
        { event: "reality.imageupdated", process: ({ detail }) => this.onImage(detail, true) },
        { event: "reality.imagelost", process: ({ detail }) => this.onImage(detail, false) },
      ],
    };
  }
}
