import { readSwitch, writeSwitch } from "./device-storage";

/**
 * The camera request of the 8th Wall engine, changed on the way (three-free, main chunk). The engine opens the camera
 * itself with `getUserMedia`; while the tracker runs, every camera request of the page passes through here.
 *
 * - **Main back lens** (Android): the engine asks for "the back camera" (`facingMode: environment`). On phones with
 *   several back lenses the browser may then pick the ultra-wide lens, which has a fixed focus – small printed
 *   triggers held close stay blurry and appear even smaller. The engine's own choice only knows the old label
 *   `camera2 0, facing back`; current Chromium browsers name the lenses `camera 0, facing back`. Camera id 0 is the
 *   main back lens on Android, so the request gets that lens's `deviceId`. Labels are only listed once the camera is
 *   allowed: without labels the camera is opened once briefly to list them. Only on Android – elsewhere (iOS "Back
 *   Camera", desktop) the request stays as it is.
 * - **Continuous focus** once the camera runs (`applyCameraDefaults()`), where the camera offers it and does not use it
 *   already – a fixed focus leaves small triggers held close blurry.
 * - **Tap to focus** (`focusAtTap()`, from the AR scene's taps that hit no entity): one focus run – at the tapped point
 *   where the browser supports `pointsOfInterest` (Chrome on the S22 does not: the whole picture is refocused) – then
 *   continuous focus again. Browsers without focus controls (Safari) ignore it.
 * - **Ask for zoom** (debug): the request gets `zoom: true` – Chrome then asks for the pan-tilt-zoom permission.
 *
 * - **Resolution**: the engine asks for exactly 960 × 720 first; a chosen resolution replaces that size (as `ideal`, so
 *   a camera without it still opens). Unset = the engine's size.
 *
 * Debug switch "lens": `browser` leaves the lens to the browser (to compare). The lens, the resolution and the zoom
 * request are stored on this device – in the app's storage (`device-storage.ts`).
 */

export type LensChoice = "main" | "browser";

const LENS_KEY = "camera-lens";
const RESOLUTION_KEY = "camera-resolution";
const ASK_ZOOM_KEY = "camera-ask-zoom";
/** Android's main back lens, in the old and the current Chromium label */
const MAIN_BACK_LENS = /^camera2? 0, facing back$/;

/** Which back lens the camera request asks for – this device, next start */
export const getLensChoice = (): LensChoice => (readSwitch(LENS_KEY) === "browser" ? "browser" : "main");

export const setLensChoice = (choice: LensChoice): void => writeSwitch(LENS_KEY, choice === "main" ? null : choice);

/** Camera sizes to choose from (landscape, 4:3 like the engine's own request); null = the engine's size */
export const CAMERA_RESOLUTIONS = [null, { width: 1280, height: 960 }, { width: 1920, height: 1440 }] as const;
export type CameraResolution = (typeof CAMERA_RESOLUTIONS)[number];

/** The camera size the request asks for – this device, next start */
export const getCameraResolution = (): CameraResolution => {
  const [width, height] = (readSwitch(RESOLUTION_KEY) ?? "").split("x").map(Number);
  return CAMERA_RESOLUTIONS.find(r => r?.width === width && r?.height === height) ?? null;
};

export const setCameraResolution = (resolution: CameraResolution): void =>
  writeSwitch(RESOLUTION_KEY, resolution ? `${resolution.width}x${resolution.height}` : null);

/** Ask for the camera's zoom when it starts (Chrome: the pan-tilt-zoom permission) – this device, next start */
export const getAskZoom = (): boolean => readSwitch(ASK_ZOOM_KEY) === "true";

export const setAskZoom = (ask: boolean): void => writeSwitch(ASK_ZOOM_KEY, ask ? "true" : null);

/** The main back lens among the cameras (Android labels), null when the labels do not name one */
export const mainBackLens = (devices: MediaDeviceInfo[]): string | null =>
  devices.find(device => device.kind === "videoinput" && MAIN_BACK_LENS.test(device.label))?.deviceId ?? null;

/** The request asks for the back camera without naming a device */
export const asksForBackCamera = (constraints: MediaStreamConstraints): boolean => {
  const video = constraints.video;
  if (!video || video === true || video.deviceId) return false;
  const facing = video.facingMode;
  const value = facing && typeof facing === "object" && !Array.isArray(facing) ? (facing.exact ?? facing.ideal) : facing;
  return [value].flat().includes("environment");
};

/** The request with the lens's `deviceId` instead of the facing mode, another size, and/or `zoom: true` (a new object) */
export const rewriteCameraRequest = (
  constraints: MediaStreamConstraints,
  options: { deviceId?: string | null; resolution?: CameraResolution; zoom?: boolean },
): MediaStreamConstraints => {
  if (!constraints.video) return constraints;
  const video: MediaTrackConstraints & { zoom?: boolean } = constraints.video === true ? {} : { ...constraints.video };
  if (options.deviceId) {
    delete video.facingMode;
    video.deviceId = { exact: options.deviceId };
  }
  if (options.resolution && (video.width || video.height)) {
    video.width = { ideal: options.resolution.width };
    video.height = { ideal: options.resolution.height };
  }
  if (options.zoom) video.zoom = true;
  return { ...constraints, video };
};

/** The lens labels are Android's; elsewhere the cameras are not listed (that would open the camera an extra time) */
const isAndroid = (): boolean => /Android/i.test(navigator.userAgent);

let originalGetUserMedia: MediaDevices["getUserMedia"] | null = null;

/** The main back lens's id; lists the cameras with labels (opening the camera once when they have none) */
const findMainBackLens = async (open: MediaDevices["getUserMedia"]): Promise<string | null> => {
  const devices = navigator.mediaDevices;
  let list = await devices.enumerateDevices();
  if (list.some(device => device.kind === "videoinput" && !device.label)) {
    (await open({ video: true })).getTracks().forEach(track => track.stop());
    list = await devices.enumerateDevices();
  }
  return mainBackLens(list);
};

/**
 * Changes the page's camera requests until `releaseCameraRequest()`: the main back lens (unless the lens switch says
 * `browser`), the chosen size and the zoom request (while "ask for zoom" is on). Call before the engine starts.
 */
export const wrapCameraRequest = (): void => {
  const devices = navigator.mediaDevices;
  if (!devices?.getUserMedia || originalGetUserMedia) return;
  const original = devices.getUserMedia.bind(devices);
  originalGetUserMedia = devices.getUserMedia;
  devices.getUserMedia = async (constraints?: MediaStreamConstraints) => {
    if (!constraints) return original(constraints);
    const back = asksForBackCamera(constraints);
    let deviceId: string | null = null;
    if (getLensChoice() === "main" && isAndroid() && back) {
      deviceId = await findMainBackLens(original).catch(() => null);
    }
    const resolution = back ? getCameraResolution() : null;
    return original(rewriteCameraRequest(constraints, { deviceId, resolution, zoom: getAskZoom() }));
  };
};

/**
 * Maps a tap on the screen to the camera frame: the picture fills the screen and is cropped like `object-fit:
 * cover`; the frame is turned to the screen's orientation (a 960 × 720 frame is shown 720 × 960 in portrait).
 */
export const tapToFramePoint = (
  tapX: number, tapY: number, viewWidth: number, viewHeight: number, frameWidth: number, frameHeight: number,
): { x: number; y: number } => {
  const portraitView = viewHeight > viewWidth;
  const [shownWidth, shownHeight] = portraitView === frameHeight > frameWidth ? [frameWidth, frameHeight] : [frameHeight, frameWidth];
  const scale = Math.max(viewWidth / shownWidth, viewHeight / shownHeight);
  const x = (tapX - (viewWidth - shownWidth * scale) / 2) / (shownWidth * scale);
  const y = (tapY - (viewHeight - shownHeight * scale) / 2) / (shownHeight * scale);
  return { x: Math.min(1, Math.max(0, x)), y: Math.min(1, Math.max(0, y)) };
};

/** How long a tap's focus run gets before continuous focus takes over again */
export const TAP_FOCUS_MS = 1500;

/** The engine's running camera (set by `applyCameraDefaults()`, cleared by `releaseCameraRequest()`) */
let cameraTrack: MediaStreamTrack | null = null;
let tapRun = 0;

type FocusCapabilities = { focusMode?: unknown; pointsOfInterest?: unknown };

const focusModes = (track: MediaStreamTrack): string[] => {
  const modes = ((track.getCapabilities?.() ?? {}) as FocusCapabilities).focusMode;
  return Array.isArray(modes) ? modes.map(String) : [];
};

/**
 * A tap on the camera picture (screen coordinates): focus there once, then continuous focus again. Resolves false when
 * the camera has no focus controls or refuses.
 */
export const focusAtTap = async (tapX: number, tapY: number): Promise<boolean> => {
  const track = cameraTrack;
  if (!track || track.readyState !== "live") return false;
  const modes = focusModes(track);
  if (!modes.includes("single-shot") && !modes.includes("continuous")) return false;
  const settings = (track.getSettings?.() ?? {}) as { width?: number; height?: number };
  const capabilities = (track.getCapabilities?.() ?? {}) as FocusCapabilities;
  const once: Record<string, unknown> = {};
  if ("pointsOfInterest" in capabilities) {
    once.pointsOfInterest = [tapToFramePoint(tapX, tapY, window.innerWidth, window.innerHeight, settings.width ?? 4, settings.height ?? 3)];
  }
  // single-shot starts a new focus run; without it, setting continuous again restarts the search
  once.focusMode = modes.includes("single-shot") ? "single-shot" : "continuous";
  const run = ++tapRun;
  try {
    await track.applyConstraints({ advanced: [once as MediaTrackConstraintSet] });
  } catch (error) {
    console.warn("[camera] Tap to focus not applied:", error);
    return false;
  }
  if (once.focusMode === "single-shot" && modes.includes("continuous")) {
    window.setTimeout(() => {
      // A newer tap or a camera change takes over
      if (run !== tapRun || cameraTrack !== track || track.readyState !== "live") return;
      track.applyConstraints({ advanced: [{ focusMode: "continuous" } as MediaTrackConstraintSet] })
        .catch(error => console.warn("[camera] Continuous focus not restored:", error));
    }, TAP_FOCUS_MS);
  }
  return true;
};

/** Defaults for the engine's running camera: continuous focus where offered and not in use (resolves either way) */
export const applyCameraDefaults = async (track: MediaStreamTrack): Promise<void> => {
  cameraTrack = track;
  const capabilities = (track.getCapabilities?.() ?? {}) as { focusMode?: unknown };
  const settings = (track.getSettings?.() ?? {}) as { focusMode?: unknown };
  if (!Array.isArray(capabilities.focusMode) || !capabilities.focusMode.includes("continuous")) return;
  if (settings.focusMode === "continuous") return;
  try {
    await track.applyConstraints({ advanced: [{ focusMode: "continuous" } as MediaTrackConstraintSet] });
  } catch (error) {
    console.warn("[camera] Continuous focus not applied:", error);
  }
};

export const releaseCameraRequest = (): void => {
  cameraTrack = null;
  tapRun++;
  if (!originalGetUserMedia) return;
  navigator.mediaDevices.getUserMedia = originalGetUserMedia;
  originalGetUserMedia = null;
};
