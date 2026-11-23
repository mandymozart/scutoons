import type { Asset } from "./api";
import { readStored, writeStored } from "#lib/storage.js";

/** Where the content's files live below the assets base (`ContentVersion::CONTENT_PREFIX`) */
const CONTENT_PREFIX = "/assets/content/";

/**
 * Where a library file belongs. The import names its files after their role (`image.jpg`, `target.jpg`, `video.mp4`)
 * in a folder per entry or scene (`/assets/content/entries/p015-1/target.jpg`), so the name alone tells nothing:
 * `place` is the folder below the collection (`p015-1`), `label` the place and the role (`p015-1 · target`).
 * An upload (or a file outside the content) has no place; its label is its name.
 */
export interface AssetPlace {
  /** `entries`, `scenes`, `plugins` … – null without a place */
  collection: string | null;
  place: string | null;
  /** The file's role, its name without the extension (`target`, `video`) – null without a place */
  role: string | null;
  label: string;
}

export function assetPlace(asset: Pick<Asset, "name" | "url">): AssetPlace {
  const path = pathOf(asset.url);
  const at = path.indexOf(CONTENT_PREFIX);
  const parts = at < 0 ? [] : path.slice(at + CONTENT_PREFIX.length).split("/").filter(Boolean).map(safeDecode);
  if (parts.length < 3) return { collection: null, place: null, role: null, label: asset.name };
  const place = parts.slice(1, -1).join("/");
  const file = parts[parts.length - 1];
  const stem = file.includes(".") ? file.slice(0, file.lastIndexOf(".")) : file;
  return { collection: parts[0], place, role: stem, label: `${place} · ${stem}` };
}

/**
 * What a card or a row calls the file: its name – an imported file under its own (`target.jpg` in a folder per entry)
 * says nothing alone, so it shows its place (`p015-1 · target`) until a name by its use was given (#298, `named`)
 */
export const assetLabel = (asset: Pick<Asset, "name" | "url" | "imported"> & { named?: boolean }): string =>
  asset.imported && !asset.named ? assetPlace(asset).label : asset.name;

/** "+2 more" when the file is used in more places than its first (#298) – "" otherwise */
export const moreUses = (asset: Pick<Asset, "place">): string => (asset.place && asset.place.more > 0 ? `+${asset.place.more} more` : "");

/** A group of the Library (#298 phase 3): a scene's files, then the experience-wide ones, then the unused ones */
export interface LibraryGroup {
  key: string;
  label: string;
  assets: Asset[];
}

/**
 * The files grouped by where they are used first (`Asset.place`): a group per scene in the scenes' order (headed by its
 * label), then "Not in a scene" (the environment, a plugin, a shared entity no target shows), then "Unused" – a group
 * only when it holds a file. Within a group by the order of use, then name.
 */
export function groupByScene(assets: readonly Asset[], scenes: readonly { id: string; label: string }[]): LibraryGroup[] {
  const titles = new Map(scenes.map(scene => [scene.id, scene.label || scene.id]));
  const byScene = new Map<string, Asset[]>();
  const experienceWide: Asset[] = [];
  const unused: Asset[] = [];
  for (const asset of assets) {
    const place = asset.place;
    if (!place) unused.push(asset);
    else if (place.scene && titles.has(place.scene)) byScene.set(place.scene, [...(byScene.get(place.scene) ?? []), asset]);
    else experienceWide.push(asset);
  }
  const sorted = (list: Asset[]) =>
    [...list].sort((a, b) => (a.place?.order ?? -1) - (b.place?.order ?? -1) || assetLabel(a).localeCompare(assetLabel(b)));
  const groups: LibraryGroup[] = [];
  for (const scene of scenes) {
    const list = byScene.get(scene.id);
    if (list) groups.push({ key: `scene:${scene.id}`, label: titles.get(scene.id) ?? scene.id, assets: sorted(list) });
  }
  if (experienceWide.length) groups.push({ key: "experience", label: "Not in a scene", assets: sorted(experienceWide) });
  if (unused.length) groups.push({ key: "unused", label: "Unused", assets: [...unused].sort((a, b) => assetLabel(a).localeCompare(assetLabel(b))) });
  return groups;
}

/**
 * The names plan in words, for the Library's *Name by use* step: `12 files get the name of their use (3 imported) –
 * page-cover.webp, p118-1_target.webp, …`; nothing to do says so
 */
export function namesPlanLine(renames: readonly { to: string; imported: boolean }[], shown = 3): string {
  if (!renames.length) return "Every used file is named by its use already.";
  const imported = renames.filter(r => r.imported).length;
  const names = renames.slice(0, shown).map(r => r.to).join(", ");
  const list = shown > 0 ? `: ${names}${renames.length > shown ? ", …" : ""}` : "";
  return `${renames.length} file${renames.length === 1 ? "" : "s"} take${renames.length === 1 ? "s" : ""} the name of ${renames.length === 1 ? "its" : "their"} use${imported ? ` (${imported} imported – their listed name)` : ""}${list}`;
}

/** The search's text: the name, the place and the collection (`p015-1`, `entries`, `target`) */
export function assetSearchText(asset: Pick<Asset, "name" | "url"> & { credits?: string | null }): string {
  const { collection, place, label } = assetPlace(asset);
  return [asset.name, label, place, collection, asset.credits].filter(Boolean).join(" ").toLowerCase();
}

/** Every word of the query in the asset's search text */
export function matchesSearch(asset: Pick<Asset, "name" | "url"> & { credits?: string | null }, query: string): boolean {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return true;
  const text = assetSearchText(asset);
  return words.every(word => text.includes(word));
}

/**
 * What the Library says when an upload's bytes were a file of it already (the API answers with that asset, named as
 * the upload names it): `heldName` is the name it had – undefined for a new file, which needs no line
 */
export const heldNotice = (asset: Pick<Asset, "name">, heldName: string | undefined): string | null =>
  heldName === undefined ? null
    : heldName === asset.name ? `${asset.name} is in the Library already – the same file, nothing new stored`
      : `${heldName} is the same file – it is now named ${asset.name}`;

/**
 * What the Library says when the image rule converted an upload (#298): the file as it came and as it is stored –
 * `Photo.jpg (6.2 MB) is stored as Photo.webp – 2048×1536, 412 kB`
 */
export const convertedNotice = (asset: Pick<Asset, "name" | "size" | "width" | "height">, from: { name: string; size: number }): string =>
  `${from.name} (${formatSize(from.size)}) is stored as ${asset.name} – ${asset.width ? `${asset.width}×${asset.height}, ` : ""}${formatSize(asset.size)}`;

/** A file's size as the library shows it: `850 kB`, `12.3 MB` */
export const formatSize = (bytes: number): string => (bytes >= 1e6 ? `${(bytes / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1e3))} kB`);

/** A video's duration as the library shows it: `0:04`, `12:30`, `1:02:03` (whole seconds, rounded) */
export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

/** The library's two views, remembered per browser */
export const LIBRARY_VIEWS = ["cards", "list"] as const;
export type LibraryView = (typeof LIBRARY_VIEWS)[number];
const VIEW_KEY = "osct-editor-library-view";

export function readLibraryView(): LibraryView {
  const stored = readStored(VIEW_KEY);
  return LIBRARY_VIEWS.includes(stored as LibraryView) ? (stored as LibraryView) : "cards";
}

export function saveLibraryView(view: LibraryView): void {
  writeStored(VIEW_KEY, view);
}

function pathOf(url: string): string {
  try {
    return new URL(url, "http://local").pathname;
  } catch {
    return url;
  }
}

/** A URL's (or a path's) file name: its last part without query or fragment, decoded – as it is when it does not decode */
export const fileNameOf = (url: string): string => safeDecode(url.split(/[?#]/)[0].split("/").pop() ?? url);

function safeDecode(part: string): string {
  try {
    return decodeURIComponent(part);
  } catch {
    return part;
  }
}
