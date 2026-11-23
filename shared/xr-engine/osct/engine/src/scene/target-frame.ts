/**
 * The frame the 8th Wall engine reads for a target image (the format of 8th Wall's image-target CLI, flat): geometry
 * and size only, no DOM and no three.js – the reader (`xr8.ts`) and `osct targets prepare` (`scripts/src/lib/recognition.ts`,
 * Node) make the same frame from it.
 */

/** Size of the luminance image the engine reads (image-target CLI: 480×640, grey) */
export const LUMINANCE_WIDTH = 480;
export const LUMINANCE_HEIGHT = 640;

/** Crop of a target image, in the orientation the engine reads it (portrait, 3:4) */
export interface XrCrop {
  left: number;
  top: number;
  width: number;
  height: number;
  /** Landscape image: turned 90° clockwise before cropping */
  isRotated: boolean;
  originalWidth: number;
  originalHeight: number;
}

/**
 * The 3:4 frame (portrait) the engine reads for an image: the **whole** image, centred, landscape images turned
 * 90° clockwise first (`isRotated`); the rest is filled with the image's edge colour. (Cropping to the centred
 * 3:4 part instead loses the top and bottom of a tall page, which is then not recognised – the image-target CLI's
 * default crop does that.) The engine gets the frame as an uncropped image.
 */
export interface TargetFrame {
  crop: XrCrop;
  /** Where the (turned) image lies in the frame, in image pixels */
  imageLeft: number;
  imageTop: number;
  /** Image width ÷ frame width, both as they lie on the page (the anchors use the image width) */
  widthFactor: number;
}

export const targetFrame = (imageWidth: number, imageHeight: number): TargetFrame => {
  const isRotated = imageWidth > imageHeight;
  const [width, height] = isRotated ? [imageHeight, imageWidth] : [imageWidth, imageHeight];
  const frameWidth = Math.max(width, Math.round((height * 3) / 4));
  const frameHeight = Math.max(height, Math.round((width * 4) / 3));
  return {
    crop: { left: 0, top: 0, width: frameWidth, height: frameHeight, isRotated, originalWidth: frameWidth, originalHeight: frameHeight },
    imageLeft: (frameWidth - width) / 2,
    imageTop: (frameHeight - height) / 2,
    widthFactor: imageWidth / (isRotated ? frameHeight : frameWidth),
  };
};

/** Grey value of an RGB pixel (Rec. 601 – what both paths write) */
export const luma = (r: number, g: number, b: number): number => Math.round(0.299 * r + 0.587 * g + 0.114 * b);

/** An image as RGBA pixels (a canvas' `ImageData`, a decoded file in Node) */
export interface RgbaPixels {
  width: number;
  height: number;
  data: Uint8Array | Uint8ClampedArray;
}

/** Mean colour of the image's outer pixel ring – what the frame is filled with (the corners count twice, as in the reader) */
const edgeColour = (image: RgbaPixels): [number, number, number] => {
  const { width, height, data } = image;
  const sum = [0, 0, 0];
  let count = 0;
  const add = (x: number, y: number) => {
    const i = (y * width + x) * 4;
    sum[0] += data[i]; sum[1] += data[i + 1]; sum[2] += data[i + 2];
    count++;
  };
  for (let x = 0; x < width; x++) { add(x, 0); add(x, height - 1); }
  for (let y = 0; y < height; y++) { add(0, y); add(width - 1, y); }
  return [Math.round(sum[0] / count), Math.round(sum[1] / count), Math.round(sum[2] / count)];
};

/** The frame of a printed image: its grey pixels (one byte each, 480 × 640) and the engine's crop */
export interface RecognitionFrame {
  grey: Uint8Array;
  width: typeof LUMINANCE_WIDTH;
  height: typeof LUMINANCE_HEIGHT;
  crop: XrCrop;
}

/**
 * A target's recognition frame made ahead (#306) from its printed image – the one builder of `osct targets prepare`
 * (Node, `scripts/src/lib/recognition.ts`) and the editor (the browser, `editor/src/lib/recognition-frame.ts`): the grey
 * 480 × 640 frame in `targetFrame()`'s geometry. Each frame pixel is the mean of the image pixels it covers (box filter),
 * blended with the edge colour where it covers the image only in part or the image is transparent.
 */
export function recognitionFrame(image: RgbaPixels): RecognitionFrame {
  const frame = targetFrame(image.width, image.height);
  const { crop } = frame;
  // The image as it lies in the frame: turned 90° clockwise when it is landscape (turned x' = H−1−y, y' = x)
  const turnedWidth = crop.isRotated ? image.height : image.width;
  const turnedHeight = crop.isRotated ? image.width : image.height;
  const at = (x: number, y: number): number => (crop.isRotated ? (image.height - 1 - x) * image.width + y : y * image.width + x) * 4;
  const [er, eg, eb] = edgeColour(image);
  const edge = luma(er, eg, eb);
  const step = crop.width / LUMINANCE_WIDTH; // image px per frame px (the frame is 3:4 like the output)
  const grey = new Uint8Array(LUMINANCE_WIDTH * LUMINANCE_HEIGHT);
  const { data } = image;
  for (let y = 0; y < LUMINANCE_HEIGHT; y++) {
    const v0 = y * step - frame.imageTop;
    const v1 = v0 + step;
    const y0 = Math.max(0, Math.floor(v0));
    const y1 = Math.min(turnedHeight, Math.ceil(v1));
    for (let x = 0; x < LUMINANCE_WIDTH; x++) {
      const u0 = x * step - frame.imageLeft;
      const u1 = u0 + step;
      const x0 = Math.max(0, Math.floor(u0));
      const x1 = Math.min(turnedWidth, Math.ceil(u1));
      let sum = 0;
      let covered = 0;
      for (let yy = y0; yy < y1; yy++) {
        const wy = Math.min(yy + 1, v1) - Math.max(yy, v0);
        if (wy <= 0) continue;
        for (let xx = x0; xx < x1; xx++) {
          const wx = Math.min(xx + 1, u1) - Math.max(xx, u0);
          if (wx <= 0) continue;
          const i = at(xx, yy);
          const alpha = data[i + 3] / 255;
          const w = wx * wy;
          sum += w * (alpha * luma(data[i], data[i + 1], data[i + 2]) + (1 - alpha) * edge);
          covered += w;
        }
      }
      const area = step * step;
      grey[y * LUMINANCE_WIDTH + x] = Math.round((sum + (area - covered) * edge) / area);
    }
  }
  return { grey, width: LUMINANCE_WIDTH, height: LUMINANCE_HEIGHT, crop };
}

/**
 * The file a recognition frame is made as – one place for the editor's builder and `osct targets prepare`: a grey,
 * lossless PNG, a working copy. The Library stores it under the image rule like every picture (the editor's upload and
 * the API turn it into WebP, #298); the release step (#304) may re-encode it like every other asset, as long as the
 * tracker can still decode it.
 */
export const RECOGNITION_FRAME_FILE = { type: "image/png", extension: "png" } as const;

/** The frame's file name in the Library: `<target>_recognition.png` */
export const recognitionFrameName = (targetId: string): string => `${targetId}_recognition.${RECOGNITION_FRAME_FILE.extension}`;
