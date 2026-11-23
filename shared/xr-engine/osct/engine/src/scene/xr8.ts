import { Matrix4, Quaternion, Vector3 } from "three";
import type { TargetRecognitionData } from "@shared/types/game-config";
import { LUMINANCE_HEIGHT, LUMINANCE_WIDTH, luma, targetFrame, type XrCrop } from "./target-frame";

export { targetFrame, type TargetFrame, type XrCrop } from "./target-frame";

/**
 * The 8th Wall engine (`@8thwall/engine`, MIT, image targets only – no SLAM): loader, the few types the app
 * uses, and image target data (what 8th Wall's image-target CLI writes): made ahead with `osct targets prepare` (a target's
 * `recognition`, #306), else made from the target image at runtime.
 * The app serves the engine's files and names their folder (`base`, ending in `/`) – the reader's `assets/xr8/`.
 */

export interface XrVector { x: number; y: number; z: number }
export interface XrQuaternion extends XrVector { w: number }

export interface XrImageTargetData {
  type: "PLANAR";
  name: string;
  imagePath: string;
  metadata: null;
  properties: XrCrop;
}

/** `imagefound` / `imageupdated` / `imagelost` detail of a flat target */
export interface XrImageDetail {
  name: string;
  position: XrVector;
  rotation: XrQuaternion;
  scale: number;
  /** Size of the (cropped) image in its own frame, × scale – the longer side is 1 */
  scaledWidth: number;
  scaledHeight: number;
}

export interface XrReality {
  position: XrVector;
  rotation: XrQuaternion;
  intrinsics: number[] | null;
}

export interface XrPipelineModule {
  name: string;
  onStart?: (args: { canvasWidth: number; canvasHeight: number }) => void;
  onUpdate?: (args: { processCpuResult: { reality?: XrReality } }) => void;
  onCanvasSizeChange?: (args: { canvasWidth: number; canvasHeight: number }) => void;
  /** "requesting", "hasStream" (with the camera `stream`), "hasVideo", "failed" (with a `reason`) */
  onCameraStatusChange?: (args: { status: string; reason?: string; stream?: MediaStream }) => void;
  onException?: (error: unknown) => void;
  listeners?: XrListener[];
}

/** What the engine's events carry (`detail`) */
export interface XrEventDetails {
  /** The targets' features are extracted: these targets can be found now */
  "reality.imagescanning": { imageTargets?: Array<{ name: string }> } | undefined;
  "reality.imagefound": XrImageDetail;
  "reality.imageupdated": XrImageDetail;
  "reality.imagelost": XrImageDetail;
}

/** A pipeline module's listener of one of the engine's events */
export type XrListener = { [E in keyof XrEventDetails]: { event: E; process: (event: { detail: XrEventDetails[E] }) => void } }[keyof XrEventDetails];

export interface XR8Api {
  version(): string;
  loadChunk(chunk: string | { name: string; url: string }): Promise<void>;
  run(config: {
    canvas: HTMLCanvasElement;
    allowedDevices?: string;
    cameraConfig?: { direction: string };
  }): void;
  pause(): void;
  resume(): void;
  isPaused(): boolean;
  stop(): void;
  addCameraPipelineModules(modules: unknown[]): void;
  clearCameraPipelineModules(): void;
  GlTextureRenderer: {
    pipelineModule(): unknown;
    /** A shader of one's own for the camera picture (`texUv`, `sampler`); the program is built again */
    configure(config: { fragmentSource?: string }): void;
  };
  XrController: {
    pipelineModule(): unknown;
    configure(config: { disableWorldTracking?: boolean; imageTargetData?: XrImageTargetData[] }): void;
    updateCameraProjectionMatrix(config: {
      cam: { pixelRectWidth: number; pixelRectHeight: number; nearClipPlane: number; farClipPlane: number };
    }): void;
  } | null;
  XrConfig: { device(): { ANY: string }; camera(): { BACK: string } };
}

/** The engine with its image tracking chunk loaded – what `loadXr8` gives */
export type XR8Tracking = XR8Api & { XrController: NonNullable<XR8Api["XrController"]> };

const hasTracking = (xr8: XR8Api): xr8 is XR8Tracking => xr8.XrController != null;

declare global {
  interface Window {
    XR8?: XR8Api;
  }
}

const ENGINE_FILES = ["xr.js", "xr-tracking.js"];

let loading: Promise<XR8Tracking> | null = null;

/** Load the engine (script tag – it is not a module) and its image tracking chunk from `base`; retried after a failure */
export const loadXr8 = (base: string): Promise<XR8Tracking> => {
  if (!loading) {
    loading = new Promise<XR8Api>((resolve, reject) => {
      if (window.XR8) return resolve(window.XR8);
      window.addEventListener(
        "xrloaded",
        () => (window.XR8 ? resolve(window.XR8) : reject(new Error("The 8th Wall engine could not be loaded"))),
        { once: true },
      );
      const script = document.createElement("script");
      script.src = `${base}xr.js`;
      script.async = true;
      script.crossOrigin = "anonymous";
      script.onerror = () => {
        script.remove();
        reject(new Error("The 8th Wall engine could not be loaded"));
      };
      document.head.appendChild(script);
    })
      .then(async XR8 => {
        await XR8.loadChunk({ name: "slam", url: new URL(`${base}xr-tracking.js`, location.href).href });
        if (!hasTracking(XR8)) throw new Error("The 8th Wall engine has no image tracking (XrController)");
        return XR8;
      })
      .catch(error => {
        loading = null;
        throw error;
      });
  }
  return loading;
};

/** Fetch the engine's files from `base` into the HTTP cache (idle time; nothing is evaluated) */
export const prefetchXr8 = (base: string): Promise<void> =>
  Promise.all(ENGINE_FILES.map(file => fetch(`${base}${file}`).then(response => response.blob()))).then(
    () => {},
    error => console.warn("[8th Wall] Could not prefetch the engine:", error),
  );

/** A target image prepared for the engine, plus the factor its anchors need */
export interface PreparedImageTarget {
  data: XrImageTargetData;
  widthFactor: number;
}

/** Mean colour of an image's outer pixel ring (fills the frame around it) */
const edgeColour = (context: CanvasRenderingContext2D, width: number, height: number): string => {
  const { data } = context.getImageData(0, 0, width, height);
  const sum = [0, 0, 0];
  let count = 0;
  const add = (x: number, y: number) => {
    const i = (y * width + x) * 4;
    sum[0] += data[i]; sum[1] += data[i + 1]; sum[2] += data[i + 2];
    count++;
  };
  for (let x = 0; x < width; x++) { add(x, 0); add(x, height - 1); }
  for (let y = 0; y < height; y++) { add(0, y); add(width - 1, y); }
  return `rgb(${sum.map(value => Math.round(value / count)).join(",")})`;
};

const dataUrlToBlob = (dataUrl: string): Blob => {
  const [header, base64] = dataUrl.split(",");
  const bytes = Uint8Array.from(atob(base64), char => char.charCodeAt(0));
  return new Blob([bytes], { type: header.slice(5).split(";")[0] });
};

/**
 * Image target data for a target image: its frame as a grey 480×640 JPEG (object URL). The URL is also
 * the target's name – the engine unloads targets by the names it is given, which are their URLs.
 */
export const makeImageTarget = async (imageSrc: string): Promise<PreparedImageTarget> => {
  const image = new Image();
  image.crossOrigin = "anonymous";
  image.src = imageSrc;
  await image.decode();
  const frame = targetFrame(image.naturalWidth, image.naturalHeight);
  const { crop } = frame;

  const turned = document.createElement("canvas");
  turned.width = crop.isRotated ? image.naturalHeight : image.naturalWidth;
  turned.height = crop.isRotated ? image.naturalWidth : image.naturalHeight;
  const turnedContext = turned.getContext("2d", { willReadFrequently: true })!;
  if (crop.isRotated) {
    turnedContext.translate(turned.width, 0);
    turnedContext.rotate(Math.PI / 2);
  }
  turnedContext.drawImage(image, 0, 0);

  const luminance = document.createElement("canvas");
  luminance.width = LUMINANCE_WIDTH;
  luminance.height = LUMINANCE_HEIGHT;
  const context = luminance.getContext("2d")!;
  context.fillStyle = edgeColour(turnedContext, turned.width, turned.height);
  context.fillRect(0, 0, LUMINANCE_WIDTH, LUMINANCE_HEIGHT);
  const scale = LUMINANCE_WIDTH / crop.width;
  context.drawImage(turned, frame.imageLeft * scale, frame.imageTop * scale, turned.width * scale, turned.height * scale);
  const pixels = context.getImageData(0, 0, LUMINANCE_WIDTH, LUMINANCE_HEIGHT);
  const data = pixels.data;
  for (let i = 0; i < data.length; i += 4) {
    const y = luma(data[i], data[i + 1], data[i + 2]);
    data[i] = data[i + 1] = data[i + 2] = y;
  }
  context.putImageData(pixels, 0, 0);
  // toDataURL, not toBlob: Chrome encodes toBlob in idle time, and the engine's frame loop leaves almost none
  // (S22: ~4 s per image while tracking, 12 ms synchronously)
  const url = URL.createObjectURL(dataUrlToBlob(luminance.toDataURL("image/jpeg", 0.92)));
  return { data: { type: "PLANAR", name: url, imagePath: url, metadata: null, properties: crop }, widthFactor: frame.widthFactor };
};

/**
 * Image target data from a target's recognition data made ahead (#306, `osct targets prepare`): the engine loads the
 * prepared grey frame by its URL – nothing to decode, draw or encode on the phone. The URL is the target's name.
 */
export const preparedImageTarget = (recognition: TargetRecognitionData): PreparedImageTarget => {
  const frame = targetFrame(recognition.width, recognition.height);
  const url = recognition.imageSrc;
  return { data: { type: "PLANAR", name: url, imagePath: url, metadata: null, properties: frame.crop }, widthFactor: frame.widthFactor };
};

const cameraInverse = new Matrix4();
const pose = new Matrix4();

/**
 * A found image's anchor relative to the camera of the frame: 1 unit = full image width, origin in the image's
 * centre (the convention the entities are placed in, `entities.ts`). World tracking is off, so only this relative
 * pose matters – the three.js camera stays at the origin.
 */
export const anchorMatrix = (camera: Pick<XrReality, "position" | "rotation">, image: XrImageDetail, widthFactor: number): Matrix4 => {
  const { position: p, rotation: r } = camera;
  cameraInverse.compose(new Vector3(p.x, p.y, p.z), new Quaternion(r.x, r.y, r.z, r.w), new Vector3(1, 1, 1)).invert();
  const scale = image.scale * image.scaledWidth * widthFactor;
  pose.compose(
    new Vector3(image.position.x, image.position.y, image.position.z),
    new Quaternion(image.rotation.x, image.rotation.y, image.rotation.z, image.rotation.w),
    new Vector3(scale, scale, scale),
  );
  return new Matrix4().multiplyMatrices(cameraInverse, pose);
};
