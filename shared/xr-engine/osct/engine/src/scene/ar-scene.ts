import { Box3, Group, type Matrix4, type Object3D, type Quaternion, SRGBColorSpace, type Texture, TextureLoader, Vector3 } from "three";
import {
  type ArError, ArSceneError, type ArSceneEvents, type ArStatus, type EntityData, ErrorCode, type GraphicsSource, type IArScene, type TextureQuality, type MediaQuality, type PassData, type SceneInspection, ArState, type Target,
} from "../types/index";
import { type AppearanceData, type ExperienceAppearance, entityMomentAppearance, type Moment, nodeMomentLook } from "@shared/types/appearance";
import { entityAssets, getAssets, getExperience, getVariableTypes, getPasses, getPlugins, getScene, getScenes, getTargets } from "../content/game-config";
import { type EnvironmentData, environmentOf } from "@shared/types/environment";
import { focusAtTap } from "../utils/camera-request";
import { Emitter } from "../utils/emitter";
import { getPreparedWindow, isGrowingWindow, scenesByDistance, subscribePreparedWindow } from "../utils/prepared-scenes";
import { AssetStore } from "./assets";
import { deviceHints, wantedTextureSize } from "./model-textures";
import { chooseMediaSrc, mediaHints } from "./media-levels";
import { isLevelledKind } from "@shared/types/media-levels";
import { type AnimationKind, celebrate, type Celebration, together } from "./celebration";
import { CARD_HOLD_MS, CardFlight } from "./card-flight";
import { buildEntity, type EntityInstance, placeEntity } from "./entities";
import { loadPlugins } from "./plugins";
import { cameraGrades, cameraGradeShader } from "./post/grade";
import { emitCoreEvent, type ProgressReader, setNavigator, setProgressReader, setSceneSource, setVariableTypes, sceneChanged, stageEnded } from "./plugin-runtime";
import { applyLogic, type LogicInstance, namedPaths, type NodeLookup } from "./behaviour";
import { logicRules } from "@shared/types/logic";
import type { IImageTracker, ImageTrackerOptions, TrackerContext } from "./tracker-types";
import { LookAround } from "./look-around";
import { Subtitles } from "./subtitles";
import { endTap, holdMedia, releaseMedia, restoreSound, setMediaSound } from "./media";
import { textureMedia } from "./textures";
import { holdFeedbacks } from "./feedback";
import { onAppUi } from "./ui";
import { ArView } from "./view";

interface Anchor {
  target: Target;
  group: Group;
  /** With the entity type it was built for (an override of another type is built anew) */
  entity: (EntityInstance & { type: string }) | null;
}

interface SceneContent {
  sceneId: string;
  targets: Target[];
  anchors: Anchor[];
}

/** What the app gives the scene: its tracker, and the graphics options the surroundings follow */
export interface ArSceneOptions {
  /** The reader: the 8th Wall `ImageTracker`; the editor: its mock tracker over the stage */
  createTracker: (context: TrackerContext) => IImageTracker;
  graphics?: GraphicsSource;
  /**
   * The reader's progress for the plugins (`api.progress`, the `experience` scope of `api.vars`): the reader passes an
   * adapter over its history, the editor its simulation – never a store (RULES #32). Without it nothing is unlocked.
   */
  progress?: ProgressReader;
  /** The app's development build: debug handles on `window` (the look-around's `osctLookAroundDebug`) */
  debug?: boolean;
  /** The language the app shows subtitles in (asked while a video plays, so a change applies at once); none = English */
  subtitleLanguage?: () => string;
  /** The app's sound setting: may videos and sounds play with sound (asked at every start); none = with sound (#290) */
  sound?: () => boolean;
  /**
   * Where a rule's `goto` goes (content schema 5.0, #334): the address (`scene:<id>`, `shared/types/address.ts`) – the
   * reader opens the scene through its router, the editor's stage switches or logs. None: a goto only logs a note.
   */
  navigate?: (address: string) => void;
}

/**
 * The editor's preview of the experience's environment (`ArScene.previewEnvironment`): the values being edited, and the
 * camera's orientation in the world (y up) that turns the world instead of the gyroscope
 */
export interface EnvironmentPreview {
  environment: EnvironmentData;
  orientation: () => Quaternion;
}

/** A tap this close to an entity's on-screen bounds still hits it (small models) */
const TAP_MARGIN_PX = 24;

/**
 * The AR scene: plain three.js + image tracking (the app's tracker, `ArSceneOptions.createTracker` – the 8th Wall
 * engine in the reader), **one** renderer and camera stream for the session.
 * Built lazily – nothing (no WebGL context, no assets, no camera) until AR first runs (RUNNING). A scene switch keeps the renderer and the camera stream and only
 *   1. stops reporting the old scene's targets (the engine keeps them loaded while they are neighbours),
 *   2. swaps the anchors, entities and assets (assets both scenes use stay),
 *   3. starts tracking the new scene's targets on the running camera.
 *
 * Every call only records the requested scene / state and queues one reconcile step; each step reads the
 * newest request, so fast switching never races and outdated scenes are skipped.
 *
 * The editor runs the same scene over a simulated tracker and its stage (`continuous` tracker): nothing in the
 * scene changes – what the author sees is what the reader's engine renders.
 */
export class ArScene implements IArScene {
  private emitter = new Emitter<ArSceneEvents>();
  private queue: Promise<void> = Promise.resolve();
  private wantedScene: string | null = null;
  private wantedState: ArState = ArState.STOPPED;
  /** The current scene's content is stale (the editor saved): the next step builds it again */
  private stale = false;

  private view: ArView | null = null;
  /** The world around the reader + its sky (3DoF, anchored to the found surfaces) (off per the reader's graphics options) */
  private lookAround: LookAround | null = null;
  private environmentPreview: EnvironmentPreview | null = null;
  private tracker: IImageTracker | null = null;
  /** Models with a texture set show their map at the size the reader's texture quality asks for on this device (#260) */
  private assets = new AssetStore(
    () => wantedTextureSize(this.textureQuality, deviceHints(this.view?.renderer.capabilities.maxTextureSize)),
    // Pictures, videos and sounds with levels (#331): the reader's media quality, read when the file loads
    data => isLevelledKind(data.assetType) ? chooseMediaSrc(data.assetType, data.src, data.levels, this.options.graphics?.getSettings().mediaQuality, mediaHints()) : data.src,
  );
  private textureQuality: TextureQuality | undefined;
  private mediaQuality: MediaQuality | undefined;
  /** The line of the playing video's subtitle file, told to the app when it changes (`subtitle`) */
  private subtitles = new Subtitles(() => this.options.subtitleLanguage?.() ?? "en", text => this.emitter.emit("subtitle", text));
  private graphicsCleanup: (() => void) | null = null;
  private content: SceneContent | null = null;
  private found = new Set<string>();
  /** Tracking runs for `content` (camera on, its targets loaded) */
  private started = false;
  private running = false;
  /**
   * The last start failed (camera denied / unavailable): no retry until the requested state changes –
   * otherwise every queued RUNNING would ask for the camera again.
   */
  private startFailed = false;
  private _status: ArStatus = "idle";
  /** Running appear / disappear animation per target id (reveal, found, lost), removed when done */
  private animations = new Map<string, Celebration>();
  /** The marginalia card in flight (#285), at most one */
  private card: { flight: CardFlight; targetId: string; landed: boolean; done: (flown: boolean) => void; timer?: number } | null = null;
  /** The entities' clock holds (the editor's Live switch off, #269): their updates get no time, the looks play on */
  private clockHeld = false;
  /** Entities shown with other values than the content's, by target id (the editor's live edit) */
  private overrides = new Map<string, EntityData>();
  /** The passes shown instead of the content's (the editor's live edit), until restored */
  private passOverride: PassData[] | null = null;
  /** The experience's looks played instead of the content's (the editor's live edit – `appearance: null` = none), until restored */
  private appearanceOverride: { appearance: ExperienceAppearance | null } | null = null;
  private postProcessing = true;
  /** The editor's viewport shading, once it set one (the reader never does) */
  private shading: string | null = null;
  private windowCleanup: (() => void) | null = null;
  /** The open scene's rules (schema 5.0): built with its entities, disposed with them */
  private sceneLogic: LogicInstance | null = null;
  /** The experience's rules (schema 5.0): live from the first scene on, across scene switches, until the stage ends */
  private experienceLogic: LogicInstance | null = null;
  /** Last told `scanning` state: tracking runs and no target is found */
  private scanning = false;

  constructor(private container: HTMLElement, private options: ArSceneOptions) {
    setProgressReader(options.progress);
    setNavigator(options.navigate);
    setSceneSource(getScene, getScenes);
    setMediaSound(options.sound);
    this.textureQuality = options.graphics?.getSettings().textureQuality;
    this.mediaQuality = options.graphics?.getSettings().mediaQuality;
  }

  get status(): ArStatus {
    return this._status;
  }

  get sceneId(): string | null {
    return this.content?.sceneId ?? null;
  }

  on<E extends keyof ArSceneEvents>(event: E, listener: ArSceneEvents[E]): () => void {
    return this.emitter.on(event, listener);
  }

  load(sceneId: string): Promise<void> {
    this.wantedScene = sceneId;
    return this.reconcile();
  }

  setState(state: ArState): Promise<void> {
    if (state !== this.wantedState) this.startFailed = false;
    this.wantedState = state;
    return this.reconcile();
  }

  refresh(): Promise<void> {
    this.stale = true;
    this.lookAround?.setEnvironment(this.environment());
    return this.reconcile();
  }

  /**
   * The editor's environment preview (not part of `IArScene` – the reader never previews): the look-around world drawn
   * with these values instead of the experience's, whatever the graphics switches say, turned by the preview's orientation;
   * `null` ends it (the experience's environment and the switches again)
   */
  previewEnvironment(preview: EnvironmentPreview | null): void {
    this.environmentPreview = preview;
    this.lookAround?.setPreview(preview ? { orientation: preview.orientation } : null);
    this.lookAround?.setEnvironment(this.environment());
  }

  /** The environment the look-around draws: the editor's preview, else the experience's */
  private environment(): EnvironmentData {
    return this.environmentPreview?.environment ?? environmentOf(getExperience());
  }

  /** The first find's animation, the reveal (called by the bridge right after `targetFound`; replaces the found one) */
  celebrate(targetId: string): void {
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    if (!anchor) return;
    this.animate(anchor, "reveal");
    anchor.entity?.onUnlocked?.();
    emitCoreEvent("targetUnlocked", { targetId });
  }

  /**
   * The marginalia card (#285): the printed picture `front` peels off the found target's page, rises, spins (showing
   * `back`, a default pattern without one) and slaps against the screen dead centre, where it becomes `reveal` (the
   * entry's own picture in full resolution) with a rumble (`card-flight.ts`). Resolves true once the impact is over (the card stays a moment longer, for the entry to open
   * over it), false at once when there is nothing to fly from
   * (the target is not found, the scene does not run, the picture does not load) or a newer flight or a teardown
   * ended it.
   */
  flyCard(targetId: string, card: { front: string; reveal?: string; back?: string }): Promise<boolean> {
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    const view = this.view;
    if (!anchor || !view || !this.running || !this.found.has(targetId)) return Promise.resolve(false);
    this.endCard();
    return new Promise(resolve => {
      const load = (src: string) => new TextureLoader().setCrossOrigin("anonymous").loadAsync(src).then(texture => {
        texture.colorSpace = SRGBColorSpace;
        return texture;
      });
      const optional = (src?: string) => (src ? load(src).catch(() => null) : Promise.resolve(null));
      void Promise.all([load(card.front), optional(card.back), optional(card.reveal)]).then(
        ([front, back, reveal]: [Texture, Texture | null, Texture | null]) => {
          const image = front.image as { width?: number; height?: number } | undefined;
          const aspect = image?.width && image.height ? image.height / image.width : 1;
          if (this.view !== view || !this.found.has(targetId) || this.card) {
            front.dispose();
            back?.dispose();
            reveal?.dispose();
            return resolve(false);
          }
          const flight = new CardFlight(anchor.group.matrix, front, aspect, back, undefined, reveal);
          view.scene.add(flight.object);
          this.card = { flight, targetId, landed: false, done: resolve };
        },
        () => resolve(false),
      );
    });
  }

  /** Card: one frame of the flight; once landed, resolve and remove it after the hold */
  private updateCard(delta: number): void {
    const card = this.card;
    if (!card || card.landed || !this.view) return;
    const anchor = this.found.has(card.targetId) ? this.content?.anchors.find(a => a.target.id === card.targetId) : undefined;
    if (card.flight.update(delta, anchor?.group.matrix ?? null, this.view.camera)) return;
    card.landed = true;
    card.done(true);
    card.timer = window.setTimeout(() => {
      if (this.card === card) this.endCard();
    }, CARD_HOLD_MS);
  }

  /** Remove the card (a flight still on its way resolves false) */
  private endCard(): void {
    const card = this.card;
    if (!card) return;
    this.card = null;
    window.clearTimeout(card.timer);
    card.flight.dispose();
    if (!card.landed) card.done(false);
  }

  sampleColor(x: number, y: number): string | null {
    return this.view?.sample(x, y) ?? null;
  }

  overrideEntity(targetId: string, entity: EntityData | null): void {
    if (entity) this.overrides.set(targetId, entity);
    else this.overrides.delete(targetId);
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    if (!anchor) return;
    this.rebuildOverride(anchor, targetId);
    // A file the scene's content does not have (the editor's library, a new node): the entity shows at once as far
    // as it can (a placeholder for a model), the file loads, then the entity is built again with it
    const missing = entityAssets(entity ?? undefined).filter(asset => !this.assets.knows(asset.id));
    if (missing.length) {
      void this.assets.load(missing).then(() => {
        if (this.overrides.get(targetId) === entity) this.rebuildOverride(anchor, targetId, true);
      });
    }
  }

  overridePasses(passes: PassData[] | null): void {
    this.passOverride = passes;
    if (this.content) this.applyPasses(this.content.sceneId);
  }

  /**
   * The editor's live edit of the experience's looks (not part of `IArScene`): the next animations play these instead of the
   * content's – `{ appearance: null }` = the experience names none (the default look everywhere); `null` = the content's again.
   * A running animation plays on.
   */
  /**
   * Hold the entities' clock (the editor's Live switch off, #269) or let it run again: while held, no entity's `update`
   * runs – components, clips, materials, textures and plugin updates stand still where they are –, the videos and sounds
   * stop where they are and a page found meanwhile does not start its video (`media.ts`, #275), and the plugins' GPU
   * feedback buffers keep their last frame. The moments' looks (reveal, found, lost) play on, so what is found still
   * appears. Let go, everything goes on from where it stood. A plugin's own timers keep theirs.
   */
  holdClock(held: boolean): void {
    this.clockHeld = held;
    holdMedia(held, [...this.assets.media(), ...textureMedia()]);
    holdFeedbacks(held);
  }

  overrideAppearance(override: { appearance: ExperienceAppearance | null } | null): void {
    this.appearanceOverride = override;
  }

  /**
   * The look a moment plays on a target's entity: its own (the components `reveal`, `found`, `lost` of its root node, as
   * overridden by the editor), else the experience's – the editor's override, else the content's (`entityMomentAppearance`)
   */
  private momentLook(kind: Moment, target: Target): AppearanceData {
    const override = this.appearanceOverride;
    return entityMomentAppearance(this.entityOf(target), override ? { appearance: override.appearance ?? undefined } : getExperience(), kind);
  }

  /**
   * The editor's viewport shading (`wireframe`, `solid`, `textured`, `rendered`), for the code on the stage: written as
   * `userData.shading` on the three.js scene, so a plugin that draws an effect of the rendered picture – a contact
   * shadow – leaves it out of the working views (docs/plugins.md "The stage's shading"). The reader never sets it: what
   * readers see is `rendered`, and no value reads the same.
   */
  setShading(mode: string): void {
    this.shading = mode;
    if (this.view) this.view.scene.userData.shading = mode;
  }

  setPostProcessing(enabled: boolean): void {
    this.postProcessing = enabled;
    this.view?.setPostProcessing(enabled);
    if (this.content) this.applyPasses(this.content.sceneId);
  }

  /**
   * A scene's passes to the view, and a grade that reaches the whole picture to the camera picture too (it lies in
   * the engine's own canvas under the view's – the tracker draws it with the grade)
   */
  private applyPasses(sceneId: string): void {
    const passes = this.passesOf(sceneId);
    this.view?.setPasses(passes);
    this.tracker?.gradeCamera?.(this.postProcessing ? cameraGradeShader(cameraGrades(passes)) : null);
  }

  /** The passes a scene shows: the editor's override or the content's (the scene's own, else the experience's) */
  private passesOf(sceneId: string): readonly PassData[] {
    return this.passOverride ?? getPasses(sceneId);
  }

  /** The anchor shows its entity as overridden: tuned in place when it can (unless `force`), built anew otherwise */
  private rebuildOverride(anchor: Anchor, targetId: string, force = false): void {
    const wanted = this.entityOf(anchor.target);
    // A running animation holds the entity's materials – it ends first
    this.animations.get(targetId)?.finish();
    this.animations.delete(targetId);
    if (!force && wanted && anchor.entity && wanted.type === anchor.entity.type && (anchor.entity.tune?.(wanted) ?? true)) {
      placeEntity(anchor.entity.object, wanted);
      return;
    }
    // Filters the entity can't change in place: build it anew with the new values
    anchor.entity?.dispose?.();
    anchor.entity?.object.removeFromParent();
    anchor.entity = this.createEntity(anchor.target);
    if (anchor.entity) {
      anchor.group.add(anchor.entity.object);
      if (this.found.has(targetId)) anchor.entity.onFound?.();
    }
  }

  /** The target's entity as shown: the editor's override or the content's */
  private entityOf(target: Target): EntityData | undefined {
    return this.overrides.get(target.id) ?? target.entity;
  }

  private createEntity(target: Target): (EntityInstance & { type: string }) | null {
    const entity = this.entityOf(target);
    const instance = entity ? buildEntity({ ...target, entity }, id => this.assets.get(id)) : null;
    return instance && entity ? { ...instance, type: entity.type } : null;
  }

  /** Start an animation of an anchor's entity (a running one of the same anchor ends first) */
  private animate(anchor: Anchor, kind: AnimationKind): void {
    const id = anchor.target.id;
    this.animations.get(id)?.finish();
    this.animations.delete(id);
    if (!anchor.entity || !this.view) return;
    // The root plays the entity's look for the moment (its own, else the experience's) over the whole tree but the nodes that name
    // a look of their own: each of those animates its subtree with it – a built-in one or a plugin's
    const data = this.entityOf(anchor.target);
    const own: Array<{ object: Object3D; look: AppearanceData }> = [];
    const find = (node: EntityData | undefined, path: string) => {
      (node?.children ?? []).forEach((child, i) => {
        const at = path ? `${path}.${i}` : `${i}`;
        const look = nodeMomentLook(child, kind);
        const object = anchor.entity?.nodes?.[at];
        if (look && object) own.push({ object, look });
        find(child, at);
      });
    };
    find(data, "");
    const apart = new Set(own.map(node => node.object));
    const root = celebrate(anchor.group, anchor.entity.object, kind, this.momentLook(kind, anchor.target), apart);
    this.animations.set(id, own.length ? together(kind, [root, ...own.map(node => celebrate(anchor.group, node.object, kind, node.look, apart))]) : root);
  }

  dispose(): Promise<void> {
    this.wantedScene = null;
    this.wantedState = ArState.STOPPED;
    return this.reconcile();
  }

  private reconcile(): Promise<void> {
    this.queue = this.queue.then(() => this.step()).catch(error => {
      console.error("[ArScene]", error);
      const message = error instanceof Error ? error.message : String(error);
      this.setStatus("error", { code: error instanceof ArSceneError ? error.code : ErrorCode.AR_FAILED, message });
    });
    return this.queue;
  }

  private setStatus(status: ArStatus, error?: ArError) {
    if (status === this._status && !error) return;
    this._status = status;
    this.emitter.emit("status", status, error);
  }

  /** Tell the app whether the scanning indicator shows: tracking runs and no target is found (`scanning` event) */
  private setScanning(scanning: boolean): void {
    if (scanning === this.scanning) return;
    this.scanning = scanning;
    this.emitter.emit("scanning", scanning);
  }

  private async step(): Promise<void> {
    if (!this.wantedScene) return this.teardown();
    // Lazy: the first RUNNING builds the renderer; until then only the request is kept
    if (!this.view && this.wantedState !== ArState.RUNNING) {
      this.setStatus("idle");
      return;
    }
    if (this.wantedScene !== this.sceneId || this.stale) {
      this.stale = false;
      await this.changeScene(this.wantedScene);
      this.startFailed = false;
    }
    switch (this.wantedState) {
      case ArState.RUNNING:
        return this.run();
      case ArState.PAUSED:
        return this.pause();
      case ArState.STOPPED:
        return this.stop();
    }
  }

  // ── Scenes ─────────────────────────────────────────────────────────────────────────────────

  /** Renderer + tracker – steps run one at a time, no race */
  private readonly trackerOptions: ImageTrackerOptions = {
    onUpdate: (id, matrix) => this.onTrackingUpdate(id, matrix),
    onSceneSeen: sceneId => {
      if (sceneId !== this.content?.sceneId) this.emitter.emit("sceneSeen", sceneId);
    },
  };

  /** The one view and its tracker, created on the first call */
  private async ensureView(): Promise<{ view: ArView; tracker: IImageTracker }> {
    if (!this.view) {
      const view = new ArView(this.container, camera => this.tracker?.fit(camera));
      this.view = view;
      view.setPostProcessing(this.postProcessing);
      if (this.shading) view.scene.userData.shading = this.shading;
      this.tracker =this.options.createTracker({ container: this.container, options: this.trackerOptions, scene: view.scene });
      view.resize();
      this.lookAround = new LookAround(view.renderer, () => this.tracker?.cameraVideo ?? null, this.options.graphics, this.environment());
      this.lookAround.debugTools = this.options.debug ?? false;
      // The reader changes the texture quality: the loaded models swap their maps; the media quality: the loaded pictures,
      // videos and sounds swap their level, a playing one goes on at the same time (#331)
      this.graphicsCleanup = this.options.graphics?.subscribe(settings => {
        if (settings.mediaQuality !== this.mediaQuality) {
          this.mediaQuality = settings.mediaQuality;
          this.assets.refreshMedia();
        }
        if (settings.textureQuality === this.textureQuality) return;
        this.textureQuality = settings.textureQuality;
        this.assets.refreshTextures();
      }) ?? null;
      if (this.environmentPreview) this.lookAround.setPreview({ orientation: this.environmentPreview.orientation });
      view.underlay = this.lookAround;
      // A simulated tracker's stage is drawn every frame (the page is always in view)
      view.needsRender = () =>
        !!this.tracker?.continuous || this.animations.size > 0 || !!this.card || !!this.content?.anchors.some(a => a.group.visible) || !!this.lookAround?.active;
      view.onFrame(delta => {
        const anchors = this.content?.anchors.filter(a => this.found.has(a.target.id)).map(a => a.group.matrix) ?? [];
        this.lookAround?.update(delta, view.camera, anchors);
        if (!this.clockHeld) this.content?.anchors.forEach(a => a.entity?.update?.(delta));
        this.subtitles.update(this.assets.videos());
        this.updateCard(delta);
        let bloom = 0;
        this.animations.forEach((animation, id) => {
          if (animation.update(delta)) {
            bloom = Math.max(bloom, animation.bloom);
            return;
          }
          this.animations.delete(id);
          // Lost and gone: hide the anchor (unless found again meanwhile)
          const anchor = animation.kind === "lost" && !this.found.has(id) && this.content?.anchors.find(a => a.target.id === id);
          if (anchor) anchor.group.visible = false;
        });
        view.bloomStrength = bloom;
      });
      document.addEventListener("click", this.onTap);
      // The window's size changed (load test budget): the tracker and the preload follow at once
      this.windowCleanup = subscribePreparedWindow(() => {
        if (!this.started || !this.content) return;
        this.prepareWindow();
        this.emitter.emit("ready", this.content.sceneId);
      });
    }
    // Created with the view: missing only when creating it failed – never a second view
    if (!this.tracker) throw new Error("AR could not start: no image tracker");
    return { view: this.view, tracker: this.tracker };
  }

  /** Bring the scene to `sceneId`: tracking stops, content is swapped, tracking restarts if it ran */
  private async changeScene(sceneId: string): Promise<void> {
    if (!getScene(sceneId)) throw new Error(`Unknown scene ${sceneId}`);
    const { view, tracker } = await this.ensureView();
    const cameraOn = tracker.hasCamera;
    this.setStatus(cameraOn ? "starting" : "loading");

    // 1. Stop tracking – the camera stream stays
    this.pauseEntities();
    this.loseAll();
    tracker.stopTracking();
    this.started = false;
    this.running = false;

    // 2. Swap the content (assets both scenes use stay loaded); the experience's plugins load with the first scene
    this.removeContent();
    const assets = getAssets(sceneId);
    this.assets.release(new Set(assets.map(a => a.id)));
    await Promise.all([this.assets.load(assets), loadPlugins(getPlugins())]);
    const targets = getTargets(sceneId);
    // The content's typed variables (schema 5.0): read again – the content may have changed
    setVariableTypes(getVariableTypes());
    // The scene scope of the plugins' variables ends before the new scene's entities are built
    sceneChanged(sceneId);
    const anchors = targets.map(target => this.buildAnchor(target));
    anchors.forEach(anchor => view.scene.add(anchor.group));
    this.content = { sceneId, targets, anchors };
    this.openLogic(sceneId);
    // The scene's passes (the previous scene's are disposed; none = the scene renders straight to the canvas)
    this.applyPasses(sceneId);

    // 3. Tracking restarts in run() (camera kept); paused / stopped it waits for the next start
    if (this.wantedState !== ArState.RUNNING) this.setStatus("ready");
  }

  private buildAnchor(target: Target): Anchor {
    const group = new Group();
    group.name = target.id;
    group.visible = false;
    group.matrixAutoUpdate = false;
    const entity = this.createEntity(target);
    if (entity) group.add(entity.object);
    return { target, group, entity };
  }

  /**
   * The rules of the scene that opened and of the experience (schema 5.0): the scene's are built for it, the experience's
   * live on (their rules read again – the content may have changed); both then listen to `sceneChanged`, now that the scene's
   * nodes exist
   */
  private openLogic(sceneId: string): void {
    const node = (targetId: string, name: string) => this.logicNode(targetId, name);
    this.sceneLogic = applyLogic(logicRules(getScene(sceneId)?.logic), { home: "scene", sceneId, node });
    const rules = logicRules(getExperience().logic);
    if (this.experienceLogic) this.experienceLogic.tune(rules);
    else this.experienceLogic = applyLogic(rules, { home: "experience", node });
    this.experienceLogic.opened();
    this.sceneLogic.opened();
  }

  /** A node of the open scene for a scene's or the experience's rule: its target's anchor, the node of that name in its tree */
  private logicNode(targetId: string, name: string): NodeLookup {
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    if (!anchor) return { missing: `target "${targetId}" is not in the open scene` };
    const data = this.entityOf(anchor.target);
    if (!data || !anchor.entity) return { missing: `target "${targetId}" has no entity` };
    const path = name ? namedPaths(data).get(name) : "";
    if (path === undefined) return { missing: `no node named "${name}" on target "${targetId}"` };
    return { object: anchor.entity.nodes?.[path] ?? (path === "" ? anchor.entity.object : undefined), timeline: anchor.entity.timelines?.[path], act: anchor.entity.actors?.[path] };
  }

  private removeContent(): void {
    this.sceneLogic?.dispose();
    this.sceneLogic = null;
    this.finishAnimations();
    this.endCard();
    this.content?.anchors.forEach(anchor => {
      anchor.entity?.onPause?.();
      anchor.entity?.dispose?.();
      anchor.group.removeFromParent();
    });
    this.content = null;
  }

  /** End running animations now (they clean up after themselves) */
  private finishAnimations(): void {
    this.animations.forEach(animation => animation.finish());
    this.animations.clear();
    if (this.view) this.view.bloomStrength = 0;
  }

  /** Tracker update of one target: move its anchor, report found / lost */
  private onTrackingUpdate(targetId: string, matrix: Matrix4 | null): void {
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    if (!anchor || !this.running) return;
    const id = anchor.target.id;
    if (matrix) {
      anchor.group.matrix.copy(matrix);
      anchor.group.matrixWorldNeedsUpdate = true;
      anchor.group.visible = true;
    }
    if (matrix && !this.found.has(id)) {
      // Found: the entity appears; the first find becomes the reveal (the bridge calls celebrate)
      this.found.add(id);
      this.animate(anchor, "found");
      anchor.entity?.onFound?.();
      this.emitter.emit("targetFound", id);
      emitCoreEvent("targetFound", { targetId: id });
    } else if (!matrix && this.found.has(id)) {
      // Lost: the entity goes at its last position, then the anchor hides
      this.found.delete(id);
      if (anchor.entity) this.animate(anchor, "lost");
      else anchor.group.visible = false;
      anchor.entity?.onLost?.();
      this.emitter.emit("targetLost", id);
      emitCoreEvent("targetLost", { targetId: id });
    }
    this.setScanning(this.found.size === 0);
  }

  /**
   * A tap on the screen: the canvas lies under the app's pages, so taps are read from the document. Taps on
   * app UI are ignored; otherwise the nearest found entity whose on-screen bounds (+ margin) contain the
   * tap is reported (`targetTapped`). A tap that hits no entity focuses the camera there (`focusAtTap()`).
   * Every tap – on app UI too – is a user gesture: videos the browser silenced get their sound (`restoreSound`, #290).
   */
  private onTap = (event: MouseEvent): void => {
    if (!this.running || !this.view) return;
    restoreSound();
    try {
      if (onAppUi(event)) return;
      if (!this.content) {
        void focusAtTap(event.clientX, event.clientY);
        return;
      }
      const hit = this.hitTest(event.clientX, event.clientY);
      if (!hit || !this.tapTarget(hit)) void focusAtTap(event.clientX, event.clientY);
    } finally {
      endTap();
    }
  };

  /**
   * A tap on a found target's entity: its hooks listen to `onTap`, the app `targetTapped` (with whether the entity used the
   * tap itself – a video paused or played, #290), the plugins the core event of the same name. The reader's tap reaches it through the hit test, an editor tool directly (`inspect().tap`).
   * False – nothing happens – while the scene does not run, or when the target is not found or has no entity.
   */
  private tapTarget(targetId: string): boolean {
    const anchor = this.content?.anchors.find(a => a.target.id === targetId);
    if (!this.running || !anchor?.entity || !this.found.has(targetId)) return false;
    const used = anchor.entity.onTap?.() === true;
    this.emitter.emit("targetTapped", targetId, used);
    emitCoreEvent("targetTapped", { targetId });
    return true;
  }

  /** The nearest found entity whose on-screen bounds (+ margin) contain the point (client px), or null */
  private hitTest(clientX: number, clientY: number): string | null {
    if (!this.view || !this.content) return null;
    const rect = this.container.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    const camera = this.view.camera;
    const box = new Box3();
    const corner = new Vector3();
    let hit: { id: string; distance: number } | null = null;
    for (const anchor of this.content.anchors) {
      if (!anchor.group.visible || !anchor.entity || !this.found.has(anchor.target.id)) continue;
      box.setFromObject(anchor.entity.object);
      if (box.isEmpty()) continue;
      let left = Infinity, right = -Infinity, top = Infinity, bottom = -Infinity;
      for (let i = 0; i < 8; i++) {
        corner.set(i & 1 ? box.max.x : box.min.x, i & 2 ? box.max.y : box.min.y, i & 4 ? box.max.z : box.min.z);
        corner.project(camera);
        const sx = ((corner.x + 1) / 2) * rect.width;
        const sy = ((1 - corner.y) / 2) * rect.height;
        left = Math.min(left, sx); right = Math.max(right, sx);
        top = Math.min(top, sy); bottom = Math.max(bottom, sy);
      }
      const inside = x >= left - TAP_MARGIN_PX && x <= right + TAP_MARGIN_PX && y >= top - TAP_MARGIN_PX && y <= bottom + TAP_MARGIN_PX;
      if (!inside) continue;
      const distance = box.getCenter(corner).distanceTo(camera.position);
      if (!hit || distance < hit.distance) hit = { id: anchor.target.id, distance };
    }
    return hit?.id ?? null;
  }

  inspect(): SceneInspection | null {
    const view = this.view;
    if (!view) return null;
    return {
      scene: view.scene,
      camera: view.camera,
      domElement: view.domElement,
      onFrame: listener => view.onFrame(listener),
      nodeObject: (targetId, path) => this.content?.anchors.find(a => a.target.id === targetId)?.entity?.nodes?.[path.join(".")] ?? null,
      nodeTimeline: (targetId, path) => this.content?.anchors.find(a => a.target.id === targetId)?.entity?.timelines?.[path.join(".")] ?? null,
      hitTest: (clientX, clientY) => this.hitTest(clientX, clientY),
      tap: targetId => this.tapTarget(targetId),
    };
  }

  private teardown(): void {
    document.removeEventListener("click", this.onTap);
    this.windowCleanup?.();
    this.windowCleanup = null;
    this.graphicsCleanup?.();
    this.graphicsCleanup = null;
    this.pauseEntities();
    this.loseAll();
    this.removeContent();
    this.assets.release();
    this.tracker?.stop();
    this.lookAround?.dispose();
    this.lookAround = null;
    this.view?.dispose();
    this.tracker = null;
    this.view = null;
    this.started = false;
    this.running = false;
    this.setScanning(false);
    // The experience's rules end with the stage (the next one starts them anew)
    this.experienceLogic?.dispose();
    this.experienceLogic = null;
    stageEnded();
    // The editor's hold ends with the scene: the next one starts with its media and buffers free
    releaseMedia();
    holdFeedbacks(false);
    this.clockHeld = false;
    this.setStatus("idle");
  }

  // ── Start / pause / stop ────────────────────────────────────────────────────────────────────

  /** Camera (if off) + tracking of the current scene's targets */
  private async startTracking(): Promise<void> {
    const { view, tracker, content } = this;
    if (!view || !tracker || !content) throw new ArSceneError(ErrorCode.AR_START_FAILED, "AR could not start: no scene loaded");
    try {
      await tracker.startCamera();
      await tracker.loadTargets(content);
    } catch (error) {
      tracker.stop();
      this.started = false;
      this.running = false;
      this.startFailed = true;
      const code = error instanceof ArSceneError ? error.code : ErrorCode.AR_START_FAILED;
      throw new ArSceneError(code, `AR could not start: ${(error as Error)?.message ?? error}`);
    }
    view.resize(); // field of view from the engine's projection
    this.started = true;
    // The scenes around this one: their targets get ready ahead (instant switch, found pages switch by themselves)
    this.prepareWindow();
    this.emitter.emit("ready", content.sceneId);
  }

  /**
   * The tracker holds the scenes next to the current one (`prepared-scenes.ts`, nearest first): in grow mode it
   * adds every other scene in idle time, nearest first; otherwise it holds the fixed ±n window (or the load test's).
   */
  private prepareWindow(): void {
    if (!this.tracker || !this.content) return;
    const tracked = (scenes: ReturnType<typeof scenesByDistance>) => scenes.map(scene => ({ sceneId: scene.id, targets: getTargets(scene.id) }));
    if (isGrowingWindow()) this.tracker.growTargets(tracked(scenesByDistance(this.content.sceneId)));
    else this.tracker.prepareTargets(tracked(getPreparedWindow(this.content.sceneId)));
  }

  private async run(): Promise<void> {
    if (!this.started) {
      if (this.startFailed) return;
      this.setStatus("starting");
      await this.startTracking();
      // A newer request arrived while starting: the next queued step applies it
      if (this.wantedState !== ArState.RUNNING || this.wantedScene !== this.sceneId) return;
    } else if (!this.running) {
      // Tracking starts afresh: targets are found again (all were lost at the pause)
      this.tracker?.resume();
    }
    this.running = true;
    this.lookAround?.start();
    this.view?.start();
    this.setScanning(this.found.size === 0);
    this.setStatus("running");
    emitCoreEvent("arState", { state: "running" });
  }

  private pause(): void {
    if (this.started && this.running) {
      this.tracker?.pause();
      this.view?.stop();
      this.running = false;
    }
    this.lookAround?.pause();
    this.pauseEntities();
    // Nothing stays where it was: the tracker drops its poses at a pause without reporting the targets lost –
    // their anchors would stay visible at the old position ("stuck in space")
    this.finishAnimations();
    this.loseAll();
    this.setScanning(false);
    this.setStatus(this.started ? "paused" : "ready");
    emitCoreEvent("arState", { state: "paused" });
  }

  private stop(): void {
    this.pauseEntities();
    this.tracker?.stop();
    this.lookAround?.pause();
    this.view?.stop();
    this.loseAll();
    this.started = false;
    this.running = false;
    this.setScanning(false);
    this.setStatus("ready");
    emitCoreEvent("arState", { state: "stopped" });
  }

  private pauseEntities(): void {
    this.content?.anchors.forEach(a => a.entity?.onPause?.());
    this.subtitles.clear();
  }

  /** Report every found target as lost (tracking stops or the targets go away) */
  private loseAll(): void {
    this.finishAnimations();
    this.content?.anchors.forEach(a => (a.group.visible = false));
    this.found.forEach(id => {
      this.emitter.emit("targetLost", id);
      emitCoreEvent("targetLost", { targetId: id });
    });
    this.found.clear();
  }
}
