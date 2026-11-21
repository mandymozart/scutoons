import { Matrix4, type Object3D, type PerspectiveCamera, Quaternion, type Raycaster, Vector3 } from "three";
import type { Target } from "@engine/types";
import { getScene } from "@engine/content/game-config";
import { EMPTY_ENGINE_STATE, publishEngineState } from "@engine/utils/tracker-debug";
import { getPreview, type PreviewOrbit, type PreviewState, type PreviewView, subscribePreview, type SurfaceGrid, updatePreview } from "./preview";
import type { IImageTracker, ImageTrackerOptions, TrackedScene } from "@engine/scene/tracker-types";
import { onAppUi } from "@engine/scene/ui";
import { type LivePlacement, type PlaceBox, placeCentre, placeHeight, surfaceAxes, surfaceLeft, type Surfaces, withLivePlacement } from "./placement";
import { DEFAULT_SURFACE, findSurface, sceneSurfaces, surfaceFrame, type SceneSurface, type SurfaceFrame, type SurfaceReference, type SurfaceReferences } from "./surfaces";

export { surfaceLeft };

/** The simulated camera's vertical field of view – about what the engine reports on a phone at 960 × 720 */
export const PREVIEW_FOV = 60;
const NEAR = 0.01;
const FAR = 100;

/** The surface a target lies on: its id (`TargetPlace.surface`, else the scene's first surface) */
export type StageSurface = string;

/** A target on the stage: its printed image, on which surface (its id) and where (mm from its top left corner) */
export interface StageTarget {
  id: string;
  imageSrc: string;
  surface: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** Switched off (`enabled: false`, #249): not in the reader's experience – the target marks draw it orange, dashed (#323) */
  off?: boolean;
}

/** The open scene as the stage draws it: its surfaces where they lie (`surfaces.ts`) and their reference images */
export interface StageScene {
  surfaces: readonly SceneSurface[];
  /** Reference images by surface id (#314): shown on their surface, else paper */
  references?: Readonly<Record<string, SurfaceReference>>;
}

export interface Orbit extends PreviewOrbit {}

/** Where the phone is held: distance from the looked-at point in mm, tilt from the vertical and yaw in degrees */
export const VIEW_PRESETS: Record<PreviewView, Orbit> = {
  reader: { distance: 350, tilt: 30, yaw: 0 },
  top: { distance: 300, tilt: 0, yaw: 0 },
  low: { distance: 320, tilt: 75, yaw: 20 },
  far: { distance: 700, tilt: 30, yaw: 0 },
};

const MIN_DISTANCE = 80;
/** Far enough to see a wall of a few metres whole */
const MAX_DISTANCE = 4000;
const MAX_TILT = 85;
/** The looked-at point may be panned this far from the scene's own (mm) */
const MAX_PAN = 600;
/** A pointer that moved this far is a drag (orbit), not a tap */
const DRAG_PX = 8;
const ORBIT_DEGREES_PER_PX = 0.4;
const TILT_DEGREES_PER_PX = 0.3;
const WHEEL_STEP = 0.08;
/** The free orbit is stored this long after the last change */
const STORE_DELAY_MS = 300;

/** Hand shake at wobble 1: a few millimetres and half a degree, slow and fast parts */
const WOBBLE_MM = 2;
const WOBBLE_DEGREES = 0.5;

const DEFAULT_PLACE_MM = 60;
const SURFACE_MARGIN_MM = 20;
const ROW_GAP_MM = 25;

/** The stage's grids from the preview (`StageGrids`): the surfaces' own grid, the floor grid and the vertical grid */
export const gridOf = (state: PreviewState): StageGrids => ({
  on: state.grid, mm: state.gridMm, origin: state.origin, axis: state.axis, surfaces: state.surfaceGrids,
  floor: state.floorGrid, floorMm: state.floorMm, wall: state.wallGrid, wallZ: state.wallZ,
});

/** What the stage draws of the grids (#316, #319): see `PreviewSettings` */
export interface StageGrids {
  /** The surfaces' own grid (the targets'): on / off, its spacing (mm), its origin, its vertical axis */
  on: boolean;
  mm: number;
  origin: PreviewState["origin"];
  axis: boolean;
  /** A surface's own grid by its id (the open scene's – #316, #314): spacing, subdivisions, on / off; a field left out: the above */
  surfaces: Readonly<Record<string, SurfaceGrid>>;
  /** The scene's floor grid on y = 0 and its spacing (mm) – the snap grid of surface places */
  floor: boolean;
  floorMm: number;
  /** The vertical grid: a plane at the scene's z = `wallZ` (mm), the floor grid's spacing */
  wall: boolean;
  wallZ: number;
}

/** The HDRI the stage shows: the chosen one, or none while it is switched off (#327) */
export const hdriOf = (state: Pick<PreviewState, "hdri" | "hdriOn">): PreviewState["hdri"] | null => (state.hdriOn ? state.hdri : null);

/** A sameness check of two grids (the stage draws them again only on a change) */
const sameGrids = (a: StageGrids, b: StageGrids): boolean => JSON.stringify(a) === JSON.stringify(b);

const degrees = (value: number) => (value * Math.PI) / 180;

/**
 * Where the open scene's targets lie on the stage. A target's own `place` (schema 3.0 – the printed book's layout,
 * #321) puts it on its surface (by id, else the scene's first). Targets without a place lie in a row across the first
 * surface (its middle when none is placed, its lower half next to placed ones).
 */
export const layoutTargets = (
  targets: readonly Target[], surfaces: ReadonlyArray<{ id: string; widthMm: number; heightMm: number }> = [],
): StageTarget[] => {
  const withImage = targets.filter(target => target.imageSrc);
  const first = surfaces[0] ?? DEFAULT_SURFACE;
  const page = { width: first.widthMm, height: first.heightMm };
  const fallback = first.id;
  const placed = (target: Target): StageTarget | null => {
    const own = target.place;
    if (!own) return null;
    const surface = findSurface(surfaces, own.surface)?.id ?? fallback;
    return { id: target.id, imageSrc: target.imageSrc, surface, x: own.x, y: own.y, width: own.width, height: own.height ?? own.width * 0.75 };
  };
  const placedTargets = withImage.map(placed);
  if (withImage.length && placedTargets.every(target => target !== null)) return placedTargets as StageTarget[];
  if (placedTargets.some(target => target !== null)) {
    // A mix: the placed ones where they are, the rest in a row across the first surface's lower half
    const rest = withImage.filter((_, i) => placedTargets[i] === null);
    const width = Math.min(60, (page.width - 20) / Math.max(1, rest.length) - 10);
    return withImage.map((target, i) => placedTargets[i] ?? {
      id: target.id, imageSrc: target.imageSrc, surface: fallback, x: 10 + rest.indexOf(target) * (width + 10), y: page.height * 0.6, width, height: width * 0.75,
    });
  }
  // A row across the middle of the first surface, shrunk to fit when there are many
  const count = withImage.length;
  const fit = Math.min(DEFAULT_PLACE_MM, (page.width - 2 * SURFACE_MARGIN_MM - (count - 1) * ROW_GAP_MM) / Math.max(1, count));
  const width = Math.max(10, fit);
  const total = count * width + (count - 1) * ROW_GAP_MM;
  const left = (page.width - total) / 2;
  return withImage.map((target, i) => ({
    id: target.id,
    imageSrc: target.imageSrc,
    surface: fallback,
    x: left + i * (width + ROW_GAP_MM),
    y: (page.height - width * 0.75) / 2,
    width,
    height: width * 0.75,
  }));
};

/**
 * The stage's frame: metres, origin at the scene's origin (a double page's spine), x to the right, y away from the
 * reader, z up. A target's centre in that frame – the centre of its printed image, whose height
 * follows the image's aspect once the stage knows it (`placement.ts`), as the engine anchors at the image's centre.
 */
export const targetCentre = (target: StageTarget, page: Surfaces = [], aspect?: number | null): Vector3 => {
  const centre = placeCentre(target, page, aspect);
  return new Vector3(centre.x / 1000, centre.y / 1000, centre.z / 1000);
};

/** How a target's image lies in the stage's frame: as its surface (flat on the table = no turn; a wall stands, #314) */
export const targetTurn = (target: StageTarget, surfaces: Surfaces = [], out = new Quaternion()): Quaternion => {
  const axes = surfaceAxes(surfaces, target.surface);
  return out.setFromRotationMatrix(new Matrix4().makeBasis(new Vector3(...axes.x), new Vector3(...axes.y), new Vector3(...axes.z)));
};

/**
 * The simulated camera in the stage's frame: on a sphere around `focus`, `tilt` degrees from the vertical towards
 * the reader (the bottom edge of the page), turned `yaw` degrees around the vertical; up = the top of the page.
 */
export const cameraMatrix = (orbit: Orbit, focus: Vector3, target = new Matrix4()): Matrix4 => {
  const tilt = degrees(orbit.tilt);
  const yaw = degrees(orbit.yaw);
  const d = orbit.distance / 1000;
  const eye = new Vector3(Math.sin(yaw) * Math.sin(tilt) * d, -Math.cos(yaw) * Math.sin(tilt) * d, Math.cos(tilt) * d).add(focus);
  const up = new Vector3(-Math.sin(yaw), Math.cos(yaw), 0);
  const rotation = new Matrix4().lookAt(eye, focus, up);
  return target.compose(eye, new Quaternion().setFromRotationMatrix(rotation), new Vector3(1, 1, 1));
};

/** Hand shake at `wobble` (0…1) at time `t` (seconds), applied to the orbit and as a sideways offset in metres */
export const wobbled = (orbit: Orbit, wobble: number, t: number): { orbit: Orbit; offset: Vector3 } => {
  if (wobble <= 0) return { orbit, offset: new Vector3() };
  return {
    orbit: {
      distance: orbit.distance + Math.sin(t * 1.3) * WOBBLE_MM * wobble,
      tilt: orbit.tilt + Math.sin(t * 2.1) * WOBBLE_DEGREES * wobble,
      yaw: orbit.yaw + Math.cos(t * 1.7) * WOBBLE_DEGREES * wobble,
    },
    offset: new Vector3(Math.sin(t * 5.3), Math.cos(t * 4.1), 0).multiplyScalar((WOBBLE_MM / 1000) * wobble),
  };
};

/** The stage's frame (z up, the reader on −y) turned into a world with y up and the scene ahead on −z: −90° about x */
const STAGE_TO_WORLD = new Quaternion().setFromAxisAngle(new Vector3(1, 0, 0), -Math.PI / 2);
const LOOK_UP = new Matrix4();

/** What the stage draws – set by the tracker, which owns the layout and the camera */
export interface StageView {
  /** The open scene: its surfaces at their places (each a board with its reference image, else paper) and the targets on them */
  setLayout(targets: readonly StageTarget[], scene: StageScene): void;
  /** The targets hidden in the tree (the eye, #250): their printed images are not drawn either */
  setHiddenPrints?(hidden: ReadonlySet<string>): void;
  /** The sky and the light: an HDRI (shown while the shading is rendered) */
  setHdri(hdri: PreviewState["hdri"] | null): void;
  /** The desk's top under the scene (the frame stays) */
  setTop?(top: PreviewState["top"]): void;
  /**
   * A translucent image where a target is about to be printed (the new-target form, the placement rows); null = none.
   * `handlesOnly`: the target itself moves with the placement (`movePrint`), the ghost adds only its handles.
   */
  setGhost?(ghost: StageTarget | null, options?: { handlesOnly?: boolean }): void;
  /** A target's printed image moves to another place (the placement being edited) – nothing else is built anew */
  movePrint?(target: StageTarget): void;
  /** A printed image's height per width, once its image has loaded (undefined before) */
  printedAspect?(targetId: string): number | undefined;
  /** The printed images on the page, each with `userData.targetId` (picking a target by its print) */
  targetPlanes?(): Object3D[];
  /** The grids: the surfaces' own (spacing per surface, counted from the origin, its vertical axis), the floor grid, the vertical grid */
  setGrid?(grid: StageGrids): void;
  /** The lines a drag snapped to (mm of the stage frame), drawn while it lasts; empty = none */
  setGuides?(guides: ReadonlyArray<{ axis: 0 | 1 | 2; at: number }>): void;
  /** The stage's frame (metres, origin at the scene's origin, z up) */
  frame?(): Object3D;
  /** The viewport's shading: the sky and the HDRI's light show only while rendered */
  setShading?(mode: PreviewState["shading"]): void;
  /** The ghost's image and corner (the placement handles) */
  ghostParts?(): { image: Object3D | null; corner: Object3D | null };
  /** Where a ray meets the scene's surfaces, in mm from the surface's top left corner, and on which surface (its id) */
  surfaceHit?(raycaster: Raycaster): { surface: StageSurface; x: number; y: number } | null;
  /**
   * The surfaces marked on the stage (#314, for the Surface mode): `outlined` wear a thin outline, `highlighted` the
   * selection's – ids of the open scene's surfaces; unknown ids are left out
   */
  setSurfaceMarks?(outlined: readonly string[], highlighted: string | null): void;
  /** The target marks (#323): every target's place framed with its id, green / orange for on / switched off */
  setTargetMarks?(on: boolean): void;
  /** Every frame: the stage's frame relative to the camera (the camera stays at the origin) */
  update(cameraInverse: Matrix4): void;
  dispose(): void;
}

/**
 * The editor's tracker: no camera, no engine. The open scene's targets lie on its surfaces (the mock stage);
 * every frame each target not switched off is reported found with the pose of its printed image relative to a
 * camera that the author moves (drag = orbit, wheel / pinch = distance, double tap = the reader's view, presets in
 * the preview panel). `ArScene` sees matrices arriving and stopping, as with the engine – reveal, unlock and outro
 * happen as in AR. The anchor convention is the engine's: 1 unit = the target's printed width, origin in its
 * centre, the image in the x/y plane.
 */
export class MockTracker implements IImageTracker {
  /** The stage is always in view: the scene renders every frame */
  readonly continuous = true;
  private started = false;
  private paused = false;
  private targets: StageTarget[] = [];
  /** The targets as laid out from the content, before a placement being edited moves one (`targets` is with it) */
  private laidOut: StageTarget[] = [];
  /** The placement being edited: that target lies there, print and entity, until it is saved or dropped */
  private livePlacement: LivePlacement | null = null;
  /** The current scene's targets as the engine handed them over (laid out again by `relayout`) */
  private tracked: readonly Target[] = [];
  private sceneId: string | null = null;
  private held: TrackedScene[] = [];
  /** Targets reported found right now */
  private found = new Set<string>();
  private stage: StageView | null = null;
  private camera: PerspectiveCamera | null = null;
  private orbit: Orbit = { ...VIEW_PRESETS.reader };
  private preview: PreviewState = getPreview();
  private unsubscribe: (() => void) | null = null;
  private frame = 0;
  private startedAt = 0;
  private storeTimer = 0;
  private pointers = new Map<number, { x: number; y: number; startX: number; startY: number; button: number }>();
  /** Two fingers: where their midpoint was (they pan together while they pinch) */
  private lastMid: { x: number; y: number } | null = null;
  /** A pointer gesture that belongs to an editor tool (the gizmo's handle under the pointer): not an orbit */
  ignorePointer: ((event: PointerEvent) => boolean) | null = null;
  private ghost: StageTarget | null = null;
  private ghostOptions: { handlesOnly?: boolean } = {};

  /** The stage's printed images (for picking) */
  targetPlanes(): Object3D[] {
    return this.stage?.targetPlanes?.() ?? [];
  }

  ghostParts(): { image: Object3D | null; corner: Object3D | null } {
    return this.stage?.ghostParts?.() ?? { image: null, corner: null };
  }

  surfaceHit(raycaster: Raycaster): { surface: StageSurface; x: number; y: number } | null {
    return this.stage?.surfaceHit?.(raycaster) ?? null;
  }

  /** Show where a target would be printed (the form's placement); `handlesOnly` while the target itself follows it */
  setGhost(ghost: StageTarget | null, options: { handlesOnly?: boolean } = {}): void {
    this.ghost = ghost;
    this.ghostOptions = options;
    this.stage?.setGhost?.(ghost, options);
  }

  /** Where a target lies on the stage as the content places it (its own place, the protocol or the row) */
  placeOf(targetId: string): StageTarget | null {
    return this.laidOut.find(target => target.id === targetId) ?? null;
  }

  /** A printed image's height per width once its image has loaded */
  printedAspect(targetId: string): number | undefined {
    return this.stage?.printedAspect?.(targetId);
  }

  /** Every printed target where it lies now (a placement being edited included), with its printed height (mm) */
  placedBoxes(): Array<{ id: string; place: StageTarget; height: number }> {
    return this.targets.map(target => ({ id: target.id, place: target, height: placeHeight(target, this.printedAspect(target.id)) }));
  }

  /** The stage's frame (null before the stage is attached) */
  stageFrame(): Object3D | null {
    return this.stage?.frame?.() ?? null;
  }

  /** Draw the lines a drag snapped to (mm of the stage frame); empty = none */
  setGuides(guides: ReadonlyArray<{ axis: 0 | 1 | 2; at: number }>): void {
    this.stage?.setGuides?.(guides);
  }

  /**
   * The placement being edited moves the target itself – its print on the page and the anchor its entity follows –
   * instead of showing a second image (`null` = back to the content's place). The content changes only on a save.
   */
  placeTarget(targetId: string | null, place: PlaceBox | null): void {
    const before = this.livePlacement?.id ?? null;
    this.livePlacement = targetId && place ? { id: targetId, place: { ...place } } : null;
    this.targets = withLivePlacement(this.laidOut, this.livePlacement);
    for (const id of new Set([before, targetId])) {
      const target = id ? this.targets.find(t => t.id === id) : undefined;
      if (target) this.stage?.movePrint?.(target);
    }
  }
  private dragging = false;
  private lastPinch = 0;
  private cameraWorld = new Matrix4();
  private cameraInverse = new Matrix4();
  /** Degrees the camera looks up from where the orbit points it (the environment's preview: the world above the scene) */
  private lookUp = 0;
  private pose = new Matrix4();

  constructor(
    private container: HTMLElement,
    private options: ImageTrackerOptions,
    private now: () => number = () => performance.now(),
  ) {}

  get hasCamera(): boolean {
    return this.started;
  }

  get tracking(): boolean {
    return this.targets.length > 0;
  }

  /** No camera picture: the look-around's sky sample and the camera panel show nothing */
  get cameraVideo(): HTMLVideoElement | null {
    return null;
  }

  get currentOrbit(): Orbit {
    return { ...this.orbit };
  }

  /** The stage that draws the page; the tracker feeds it the layout and the camera */
  attachStage(stage: StageView): void {
    this.stage?.dispose();
    this.stage = stage;
    if (this.sceneId) stage.setLayout(this.targets, this.stageScene());
    stage.setShading?.(this.preview.shading);
    stage.setHdri(hdriOf(this.preview));
    stage.setTop?.(this.preview.top);
    stage.setGrid?.(gridOf(this.preview));
    stage.setGhost?.(this.ghost, this.ghostOptions);
    stage.setSurfaceMarks?.(this.marks.outlined, this.marks.highlighted);
  }

  private stageScene(): StageScene {
    const references = this.sceneId ? this.references[this.sceneId] : undefined;
    return { surfaces: this.surfaces, ...(references ? { references } : {}) };
  }

  /** The open scene's surfaces where they lie (#314, `place`) */
  private surfaces: SceneSurface[] = [];
  /** …and in the stage frame (the placement math, the anchors, the hits) */
  private frames: SurfaceFrame[] = [];

  /** The open scene's surfaces in the stage frame (mm) – empty before a scene is open */
  surfaceFrames(): readonly SurfaceFrame[] {
    return this.frames;
  }

  /** The open scene's surfaces with their places in the scene (#314: the surface gizmo sits on one) */
  sceneSurfaces(): readonly SceneSurface[] {
    return this.surfaces;
  }

  /** Where the surfaces lie for the placement math: the scene's frames (none: the stand-in double page) */
  private placeSurfaces(): Surfaces {
    return this.frames;
  }

  /** The reference images (#314) by scene id and surface id, set by the page; the open scene's lie on its surfaces */
  private references: SurfaceReferences = {};

  setSurfaceReferences(references: SurfaceReferences): void {
    this.references = references;
    this.relayout();
  }

  /** Outline / highlight surfaces by id on the stage (the Surface mode) */
  setSurfaceMarks(outlined: readonly string[], highlighted: string | null): void {
    this.marks = { outlined: [...outlined], highlighted };
    this.stage?.setSurfaceMarks?.(outlined, highlighted);
  }
  private marks: { outlined: string[]; highlighted: string | null } = { outlined: [], highlighted: null };

  async startCamera(): Promise<void> {
    if (this.started) return;
    this.started = true;
    this.startedAt = this.now();
    this.applyPreview(getPreview(), true);
    this.unsubscribe = subscribePreview(state => this.applyPreview(state, false));
    this.listen(true);
    this.loop();
    this.publishState();
  }

  async loadTargets({ sceneId, targets }: TrackedScene): Promise<void> {
    if (!this.started) throw new Error("loadTargets() needs the camera");
    this.stopTracking();
    this.sceneId = sceneId;
    this.tracked = targets;
    this.relayout();
  }

  /**
   * Lay the open scene out again from the content as it is now – its surfaces where they lie – without loading its
   * targets anew: the engine does not reload a scene that is already tracked.
   */
  relayout(): void {
    if (!this.sceneId) return;
    const scene = getScene(this.sceneId);
    this.surfaces = sceneSurfaces(scene);
    this.frames = this.surfaces.map(surfaceFrame);
    // The editor loads the switched-off targets too (`keepSwitchedOffTargets`): the target marks tell them apart (#323)
    const off = new Set(this.tracked.filter(target => target.enabled === false).map(target => target.id));
    this.laidOut = layoutTargets(this.tracked, this.frames).map(target => (off.has(target.id) ? { ...target, off: true } : target));
    this.targets = withLivePlacement(this.laidOut, this.livePlacement);
    this.stage?.setLayout(this.targets, this.stageScene());
    this.stage?.setSurfaceMarks?.(this.marks.outlined, this.marks.highlighted);
    this.publishState();
  }

  prepareTargets(scenes: readonly TrackedScene[]): void {
    this.held = [...scenes];
    this.publishState();
  }

  growTargets(scenes: readonly TrackedScene[]): void {
    this.held = [...scenes];
    this.publishState();
  }

  pause(): void {
    if (!this.started || this.paused) return;
    this.paused = true;
    this.found.clear(); // as the engine: nothing is reported lost at a pause, the scene drops its poses itself
    cancelAnimationFrame(this.frame);
    this.publishState();
  }

  resume(): void {
    if (!this.started || !this.paused) return;
    this.paused = false;
    this.loop();
  }

  stopTracking(): void {
    this.found.forEach(id => this.options.onUpdate(id, null));
    this.found.clear();
    this.targets = [];
    this.laidOut = [];
    this.sceneId = null;
  }

  stop(): void {
    this.stopTracking();
    cancelAnimationFrame(this.frame);
    window.clearTimeout(this.storeTimer);
    this.listen(false);
    this.unsubscribe?.();
    this.unsubscribe = null;
    this.stage?.dispose();
    this.stage = null;
    this.held = [];
    this.started = false;
    this.paused = false;
    publishEngineState(EMPTY_ENGINE_STATE);
  }

  /** Look up from the orbit's direction by this many degrees (0 = at the scene); the targets stay found */
  setLookUp(degrees: number): void {
    this.lookUp = Math.min(90, Math.max(-30, degrees));
  }

  /**
   * The camera's orientation in a world with y up (the stage's +z), as a phone's gyroscope would report it: what turns
   * the engine's look-around world in the environment's preview
   */
  worldOrientation(out = new Quaternion()): Quaternion {
    return out.setFromRotationMatrix(this.cameraWorld).premultiply(STAGE_TO_WORLD);
  }

  /** A fixed field of view; the aspect follows the container, as the engine's intrinsics would */
  fit(camera: PerspectiveCamera): void {
    this.camera = camera;
    camera.fov = PREVIEW_FOV;
    const width = this.container.clientWidth;
    const height = this.container.clientHeight;
    if (width && height) camera.aspect = width / height;
    camera.near = NEAR;
    camera.far = FAR;
    camera.updateProjectionMatrix();
  }

  /** One frame at `nowMs`: the camera from the orbit (+ wobble), every visible target's anchor, the stage */
  tick(nowMs: number): void {
    if (!this.started || this.paused) return;
    const t = (nowMs - this.startedAt) / 1000;
    const { orbit, offset } = wobbled(this.orbit, this.preview.wobble, t);
    cameraMatrix(orbit, this.focus(), this.cameraWorld);
    if (offset.lengthSq() > 0) this.cameraWorld.setPosition(new Vector3().setFromMatrixPosition(this.cameraWorld).add(offset));
    if (this.lookUp) this.cameraWorld.multiply(LOOK_UP.makeRotationX((this.lookUp * Math.PI) / 180));
    this.cameraInverse.copy(this.cameraWorld).invert();
    const hidden = new Set(this.preview.hidden);
    for (const target of this.targets) {
      if (hidden.has(target.id)) {
        if (this.found.delete(target.id)) this.options.onUpdate(target.id, null);
        continue;
      }
      const scale = target.width / 1000;
      const surfaces = this.placeSurfaces();
      this.pose.compose(targetCentre(target, surfaces, this.stage?.printedAspect?.(target.id)), targetTurn(target, surfaces), new Vector3(scale, scale, scale));
      const matrix = new Matrix4().multiplyMatrices(this.cameraInverse, this.pose);
      if (!this.found.has(target.id)) {
        this.found.add(target.id);
        this.publishState();
      }
      this.options.onUpdate(target.id, matrix);
    }
    this.stage?.setHiddenPrints?.(hidden);
    this.stage?.update(this.cameraInverse);
  }

  /**
   * The point the camera looks at: the scene's origin (a double page's spine) plus the author's pan. Never a target's
   * centre: a scene change or targets arriving would move the camera (#105)
   */
  private focus(): Vector3 {
    const pan = this.orbit.pan;
    return new Vector3((pan?.x ?? 0) / 1000, (pan?.y ?? 0) / 1000, 0);
  }

  private loop(): void {
    cancelAnimationFrame(this.frame);
    const step = () => {
      if (!this.started || this.paused) return;
      this.tick(this.now());
      this.frame = requestAnimationFrame(step);
    };
    this.frame = requestAnimationFrame(step);
  }

  private applyPreview(state: PreviewState, initial: boolean): void {
    const previous = this.preview;
    this.preview = state;
    if (initial || state.view !== previous.view || (state.orbit === null && previous.orbit !== null)) {
      this.orbit = { ...(state.orbit ?? VIEW_PRESETS[state.view]) };
    } else if (state.orbit && !sameOrbit(state.orbit, this.orbit)) {
      this.orbit = { ...state.orbit };
    }
    if (initial || state.shading !== previous.shading) this.stage?.setShading?.(state.shading);
    if (initial || state.hdri !== previous.hdri || state.hdriOn !== previous.hdriOn) this.stage?.setHdri(hdriOf(state));
    if (initial || state.top !== previous.top) this.stage?.setTop?.(state.top);
    if (initial || !sameGrids(gridOf(state), gridOf(previous))) this.stage?.setGrid?.(gridOf(state));
    if (initial || state.targetMarks !== previous.targetMarks) this.stage?.setTargetMarks?.(state.targetMarks);
    if (state.hidden !== previous.hidden) this.publishState();
  }

  /** The free orbit, stored once the author pauses */
  private orbitChanged(): void {
    window.clearTimeout(this.storeTimer);
    this.storeTimer = window.setTimeout(() => updatePreview({ orbit: { ...this.orbit } }), STORE_DELAY_MS);
  }

  private zoom(factor: number): void {
    this.orbit.distance = Math.min(MAX_DISTANCE, Math.max(MIN_DISTANCE, this.orbit.distance * factor));
    this.orbitChanged();
  }

  /** Move the looked-at point by a pointer movement (px): the stage follows the pointer in the page plane */
  private pan(dx: number, dy: number): void {
    const height = this.container.clientHeight || window.innerHeight || 1;
    const mmPerPx = (2 * this.orbit.distance * Math.tan(degrees(PREVIEW_FOV) / 2)) / height;
    const yaw = degrees(this.orbit.yaw);
    // The screen's right and up in the page plane (the camera's up is the top of the page, turned by the yaw)
    const right = { x: Math.cos(yaw), y: Math.sin(yaw) };
    const up = { x: -Math.sin(yaw), y: Math.cos(yaw) };
    const pan = this.orbit.pan ?? { x: 0, y: 0 };
    this.orbit.pan = {
      x: Math.min(MAX_PAN, Math.max(-MAX_PAN, pan.x - (right.x * dx - up.x * dy) * mmPerPx)),
      y: Math.min(MAX_PAN, Math.max(-MAX_PAN, pan.y - (right.y * dx - up.y * dy) * mmPerPx)),
    };
    this.orbitChanged();
  }

  // ── Input: from the document (a drag goes on outside the stage), started only on the stage, never on the app's UI ──

  /** A gesture starts on the stage: inside the container and not on a panel, bar, menu or control (#131) */
  private onStage(event: Event): boolean {
    return !onAppUi(event) && event.target instanceof Node && this.container.contains(event.target);
  }

  private listen(on: boolean): void {
    const capture: AddEventListenerOptions = { capture: true };
    const listeners: Array<[string, EventListener, AddEventListenerOptions]> = [
      ["pointerdown", this.onPointerDown as EventListener, capture],
      ["pointermove", this.onPointerMove as EventListener, capture],
      ["pointerup", this.onPointerUp as EventListener, capture],
      ["pointercancel", this.onPointerUp as EventListener, capture],
      ["click", this.onClick as EventListener, capture],
      ["dblclick", this.onDoubleClick as EventListener, capture],
      ["wheel", this.onWheel as EventListener, { capture: true, passive: false }],
    ];
    listeners.forEach(([type, listener, options]) =>
      on ? document.addEventListener(type, listener, options) : document.removeEventListener(type, listener, options));
  }

  private onPointerDown = (event: PointerEvent): void => {
    if (this.paused || !this.onStage(event) || this.ignorePointer?.(event)) return;
    this.pointers.set(event.pointerId, { x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY, button: event.button });
    if (this.pointers.size === 1) this.dragging = false;
    this.lastPinch = 0;
    this.lastMid = null;
    if (event.button === 1) event.preventDefault(); // the middle button pans; no autoscroll
  };

  private onPointerMove = (event: PointerEvent): void => {
    const pointer = this.pointers.get(event.pointerId);
    if (!pointer) return;
    if (this.ignorePointer?.(event)) {
      // The gizmo took the gesture after all (touch: no hover before the press)
      this.pointers.delete(event.pointerId);
      return;
    }
    if (this.pointers.size === 2) {
      const [a, b] = [...this.pointers.values()];
      const other = a === pointer ? b : a;
      const distance = Math.hypot(event.clientX - other.x, event.clientY - other.y);
      if (this.lastPinch) this.zoom(this.lastPinch / distance);
      this.lastPinch = distance;
      const mid = { x: (event.clientX + other.x) / 2, y: (event.clientY + other.y) / 2 };
      if (this.lastMid) this.pan(mid.x - this.lastMid.x, mid.y - this.lastMid.y);
      this.lastMid = mid;
      this.dragging = true;
    } else {
      const dx = event.clientX - pointer.x;
      const dy = event.clientY - pointer.y;
      if (!this.dragging && Math.hypot(event.clientX - pointer.startX, event.clientY - pointer.startY) < DRAG_PX) {
        pointer.x = event.clientX;
        pointer.y = event.clientY;
        return;
      }
      this.dragging = true;
      if (pointer.button === 1 || event.shiftKey) {
        // Shift + drag or the middle button: pan (as in every 3D tool); plain drag: orbit
        this.pan(dx, dy);
      } else {
        this.orbit.yaw += dx * ORBIT_DEGREES_PER_PX;
        this.orbit.tilt = Math.min(MAX_TILT, Math.max(0, this.orbit.tilt + dy * TILT_DEGREES_PER_PX));
        this.orbitChanged();
      }
    }
    pointer.x = event.clientX;
    pointer.y = event.clientY;
  };

  private onPointerUp = (event: PointerEvent): void => {
    this.pointers.delete(event.pointerId);
    this.lastPinch = 0;
    this.lastMid = null;
  };

  /** A click that ends a drag is no tap on the scene (ArScene listens on the document in the bubble phase) */
  private onClick = (event: MouseEvent): void => {
    if (!this.dragging) return;
    this.dragging = false;
    event.stopPropagation();
    event.preventDefault();
  };

  private onDoubleClick = (event: MouseEvent): void => {
    if (this.paused || !this.onStage(event)) return;
    updatePreview({ view: "reader", orbit: null });
  };

  private onWheel = (event: WheelEvent): void => {
    if (this.paused || !this.onStage(event)) return;
    event.preventDefault();
    this.zoom(1 + Math.sign(event.deltaY) * WHEEL_STEP);
  };

  /** The debug overlay's engine view: the scene "held" with all its targets ready, found = reported */
  private publishState(): void {
    if (!this.started) return;
    const heldScenes = this.held.filter(scene => scene.sceneId !== this.sceneId);
    publishEngineState({
      current: this.sceneId ? { sceneId: this.sceneId, targets: this.targets.length } : null,
      held: heldScenes.map(scene => ({ sceneId: scene.sceneId, targets: scene.targets.length })),
      preparing: [],
      targets: this.targets.length + heldScenes.reduce((n, scene) => n + scene.targets.length, 0),
      ready: this.targets.length + heldScenes.reduce((n, scene) => n + scene.targets.length, 0),
      extractingSince: null,
      budget: null,
      growing: null,
      seen: [...this.found],
      lastSwitch: null,
    });
  }
}

const sameOrbit = (a: Orbit, b: Orbit) =>
  a.distance === b.distance && a.tilt === b.tilt && a.yaw === b.yaw && (a.pan?.x ?? 0) === (b.pan?.x ?? 0) && (a.pan?.y ?? 0) === (b.pan?.y ?? 0);
