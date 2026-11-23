import { CanvasTexture, LinearFilter, NoColorSpace, Vector2 } from "three";

/** Width of the camera copy the sky test reads, in pixels */
const SAMPLE_WIDTH = 160;

/**
 * The camera picture for the sky test: the engine's video drawn into a small canvas (same aspect), refreshed every
 * other frame. The sky's edges need no more detail, and uploading the full video every frame costs too much.
 */
export class SkySample {
  /** How much of the video the screen shows per axis (the engine draws it to cover the screen) */
  readonly cover = new Vector2(1, 1);
  /** One pixel of the sample, in texture coordinates */
  readonly texel = new Vector2(1 / SAMPLE_WIDTH, 1 / SAMPLE_WIDTH);
  private canvas: HTMLCanvasElement | null = null;
  private texture: CanvasTexture | null = null;
  private video: HTMLVideoElement | null = null;
  private frame = 0;

  constructor(private cameraVideo: () => HTMLVideoElement | null) {}

  /** The sample for this frame, or null while the camera shows no picture */
  update(screenAspect: number): CanvasTexture | null {
    const video = this.cameraVideo();
    if (!video || video.readyState < 2 || !video.videoWidth) return null;
    if (video !== this.video || !this.canvas || !this.texture) this.prepare(video);
    if (this.frame++ % 2 === 0) {
      this.canvas!.getContext("2d")?.drawImage(video, 0, 0, this.canvas!.width, this.canvas!.height);
      this.texture!.needsUpdate = true;
    }
    const videoAspect = video.videoWidth / video.videoHeight;
    this.cover.set(Math.min(1, screenAspect / videoAspect), Math.min(1, videoAspect / screenAspect));
    return this.texture;
  }

  private prepare(video: HTMLVideoElement): void {
    this.video = video;
    this.canvas ??= document.createElement("canvas");
    this.canvas.width = SAMPLE_WIDTH;
    this.canvas.height = Math.round((SAMPLE_WIDTH * video.videoHeight) / video.videoWidth);
    this.texel.set(1 / this.canvas.width, 1 / this.canvas.height);
    this.texture?.dispose();
    this.texture = new CanvasTexture(this.canvas);
    this.texture.colorSpace = NoColorSpace; // raw sRGB values: the sky test's thresholds are in those
    this.texture.minFilter = LinearFilter;
    this.texture.generateMipmaps = false;
    this.frame = 0;
  }

  dispose(): void {
    this.texture?.dispose();
    this.texture = null;
    this.video = null;
  }
}
