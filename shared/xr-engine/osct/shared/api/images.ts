/**
 * The image rule (#298, docs/content.md "The image rule"): one way for every picture into the experience. The API converts an
 * uploaded image (jpg, png, webp) to WebP at upload (server/api/src/Images.php, the numbers mirrored as `Assets::IMAGE_*`
 * and pinned by mcp/test/php-rules.test.ts), the editor and the MCP server name the use where they know it (`?fit=`),
 * and the content build writes the same sizes (scripts/src/lib/optimize-media.ts, WebP behind `OSCT_IMAGE_FORMAT=webp`).
 *
 * - the longest side at most `IMAGE_MAX_SIDE[fit]` px – never scaled up;
 * - WebP at `IMAGE_WEBP_QUALITY`, lossless when the image has transparency (an icon, a cut-out);
 * - a JPEG's EXIF orientation applied, all metadata dropped (EXIF, XMP, ICC);
 * - an image above `IMAGE_MAX_PIXELS` is refused before it is decoded.
 *
 * Not in the content contract: the rule says how files are made, not what a configuration holds.
 */

/** The longest side per use, in px. `default` = an upload whose use is not known yet (the Library) */
export const IMAGE_MAX_SIDE = {
  /** A surface's reference image (#314) – shown over the whole surface in the editor */
  reference: 2048,
  /** An entry's colour picture – the card flip shows it full screen */
  entryImage: 2048,
  /** A target's printed image – what the image tracker learns; more detail only costs memory */
  target: 1200,
  /** A model's map at one of its sizes (#260, #331) – 512 … 4096 px, square; the largest level keeps its 4096 */
  texture: 4096,
  /** Every other Library image (a sky, a picture on a node) */
  default: 2048,
} as const;

/** The uses an upload can name: `POST …/assets?fit=<use>` */
export type ImageFit = keyof typeof IMAGE_MAX_SIDE;
export const IMAGE_FITS = Object.keys(IMAGE_MAX_SIDE) as ImageFit[];

/** WebP quality of a lossy image (0–100) */
export const IMAGE_WEBP_QUALITY = 82;

/** The most pixels an upload may have before it is decoded (width × height) – 50 MP, e.g. 8,660 × 5,773 */
export const IMAGE_MAX_PIXELS = 50_000_000;

/** The size an image of `width` × `height` gets under the rule for `fit` (never larger than it is) */
export function fitImage(width: number, height: number, fit: ImageFit = "default"): { width: number; height: number } {
  const scale = Math.min(1, IMAGE_MAX_SIDE[fit] / Math.max(width, height));
  return scale < 1 ? { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) } : { width, height };
}

/**
 * An upload's use where the caller knows it (#298): the size rule (`fit`) and what names the file – the entry of a target
 * image or a colour picture (`<entry>_target.webp`, `<entry>_image.webp`), the scene and surface of a reference image
 * (`<scene>_<surface>.webp` – `AssetNames::uploadStem`)
 */
export interface ImageUse {
  fit: ImageFit;
  entry?: string;
  surface?: { scene: string; surface: string };
}

/** The query of an upload for its use: `?fit=target&entry=p118-1`, `?fit=reference&scene=s118-119&surface=surface` ("" for none) */
export function imageUseQuery(use: ImageFit | ImageUse | undefined): string {
  if (!use) return "";
  const { fit, entry, surface } = typeof use === "string" ? { fit: use, entry: undefined, surface: undefined } : use;
  const params = new URLSearchParams();
  if (fit !== "default") params.set("fit", fit);
  if (entry && (fit === "target" || fit === "entryImage")) params.set("entry", entry);
  if (surface && fit === "reference") {
    params.set("scene", surface.scene);
    params.set("surface", surface.surface);
  }
  const query = params.toString();
  return query ? `?${query}` : "";
}
