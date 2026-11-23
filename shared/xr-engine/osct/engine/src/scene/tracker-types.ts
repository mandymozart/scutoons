import type { Matrix4, PerspectiveCamera, Scene } from "three";
import type { Target } from "../types/index";

/** A camera that sends no picture within this time is reported as not responding */
export const CAMERA_START_TIMEOUT_MS = 10000;

/** What the tracker needs to track a scene: its targets (the engine's image targets are made from their images) */
export interface TrackedScene {
  sceneId: string;
  targets: readonly Target[];
}

export interface ImageTrackerOptions {
  /**
   * The anchor matrix of a target (1 unit = target width, origin in the target's centre, the image in the
   * x/y plane), null when the target is lost
   */
  onUpdate: (targetId: string, matrix: Matrix4 | null) => void;
  /**
   * A target of a prepared scene (`prepareTargets()`) is seen steadily while none of the current scene's is –
   * the reader has turned the page
   */
  onSceneSeen?: (sceneId: string) => void;
}

/** What the scene hands the app's tracker factory (`ArSceneOptions.createTracker`) */
export interface TrackerContext {
  /** The scene's container element (the camera picture goes behind it; pointer input comes from it) */
  container: HTMLElement;
  options: ImageTrackerOptions;
  /** The three.js scene – a simulated tracker may add its stage (the table and the surfaces) to it */
  scene: Scene;
}

/**
 * Camera + image tracking, without a renderer (`ArScene` renders). The camera stream survives a scene
 * switch: `loadTargets()` replaces only the tracked targets. Implementations: `ImageTracker` (8th Wall engine,
 * `xr8-tracker.ts`) in the reader, the editor's mock tracker over its stage.
 */
export interface IImageTracker {
  readonly hasCamera: boolean;
  readonly tracking: boolean;
  /** The scene renders every frame while this tracker runs (a simulated camera: its stage is always in view) */
  readonly continuous?: boolean;
  /** The camera stream's video element (for effects that read the picture), while the camera runs */
  readonly cameraVideo: HTMLVideoElement | null;
  /** Request the back camera; throws when it is unavailable or sends no picture */
  startCamera(): Promise<void>;
  /** Track the targets of a scene on the running camera (replaces the previous targets) */
  loadTargets(scene: TrackedScene): Promise<void>;
  /**
   * Scenes the reader may switch to next (the neighbours): the tracker may get their targets ready ahead, so
   * `loadTargets()` for one of them starts at once. Called after the current scene is tracked.
   */
  prepareTargets(scenes: readonly TrackedScene[]): void;
  /**
   * Grow mode: add these scenes' targets to what the tracker holds, in this order (nearest first), a few at a time
   * in idle time, each batch after the previous one is ready – nothing already held is dropped. A newer call
   * replaces the order (the reader switched scenes); `prepareTargets()` ends the growth.
   */
  growTargets(scenes: readonly TrackedScene[]): void;
  /** Tracking and the camera picture pause, the stream stays */
  pause(): void;
  resume(): void;
  stopTracking(): void;
  /** Release the camera (and the tracking) */
  stop(): void;
  /**
   * Draw the camera picture with this fragment shader (`cameraGradeShader`: a grade that reaches the whole picture),
   * `null` = as the camera gives it. Kept across a camera restart. A tracker without a picture of its own (the
   * editor's stage) has none – its desk is graded in the post chain.
   */
  gradeCamera?(fragmentShader: string | null): void;
  /** Fit the camera picture to the container and give the three.js camera the matching projection */
  fit(camera: PerspectiveCamera): void;
}
