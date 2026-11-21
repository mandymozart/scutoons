/**
 * Gzip copies of binary content: the content build writes `<file>.gz` next to every `.glb` (models) – hosts
 * serve this type uncompressed.
 * Browsers with DecompressionStream (Chrome 80+, Safari 16.4+) load the `.gz` and unpack it; others – or a
 * missing `.gz` – load the original. The preloader and the AR scene use the same URL, so a preload is a
 * cache hit.
 */

import { contentPathOf, contentUrl, isContentUrl } from "./game-config";

/** File types the content build writes a `.gz` copy for (scripts/src/lib/optimize-media.ts) */
const GZIPPED = /\.glb$/i;

export const supportsGzip = (): boolean => typeof DecompressionStream === "function";

/** Same-origin paths and the content CDN have the `.gz` copies; external URLs do not */
const isGzipped = (src: string): boolean =>
  GZIPPED.test(src.split(/[?#]/)[0]) && (isContentUrl(src) || !/^[a-z]+:\/\//i.test(src));

/** The `.gz` copy's URL (content URLs through `contentUrl`) */
const gzUrl = (src: string): string => {
  const path = contentPathOf(src);
  return path ? contentUrl(`${path}.gz`) : `${src}.gz`;
};

/** The URL to download for a content file (its `.gz` copy where there is one and the browser can unpack it) */
export const compressedUrl = (src: string): string => (isGzipped(src) && supportsGzip() ? gzUrl(src) : src);

const fetchBuffer = async (url: string): Promise<ArrayBuffer> => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${response.status} ${response.statusText} (${url})`);
  return response.arrayBuffer();
};

/** Gzip data starts with 1f 8b (a .glb never does) */
const isGzip = (bytes: Uint8Array): boolean => bytes[0] === 0x1f && bytes[1] === 0x8b;

/**
 * The file's bytes (unpacked from the `.gz` where possible). Some servers send `.gz` files with
 * `Content-Encoding: gzip` (the Vite dev server, often Apache) – the browser has unpacked them already.
 */
export const loadCompressed = async (src: string): Promise<ArrayBuffer> => {
  if (compressedUrl(src) !== src) {
    try {
      const response = await fetch(compressedUrl(src));
      if (response.ok) {
        const bytes = new Uint8Array(await response.arrayBuffer());
        if (!isGzip(bytes)) return bytes.buffer; // already unpacked by the browser
        const unpacked = new Response(bytes).body!.pipeThrough(new DecompressionStream("gzip"));
        return await new Response(unpacked).arrayBuffer();
      }
    } catch (error) {
      console.warn(`[compressed] ${src}.gz could not be loaded, using ${src}:`, error);
    }
  }
  return fetchBuffer(src);
};
