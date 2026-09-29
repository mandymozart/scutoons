const trim = (url: string) => url.replace(/\/+$/, '');

/**
 * Backend API (accounts, admin API). `VITE_API_URL` (dev http://localhost:9000/api, or :9100 while the old repo still
 * serves :9000); prod fallback https://qr.scutoons.com/api until api.scutoons.com exists. `VITE_QR_API` = old name.
 */
export const API_URL: string = trim(
	import.meta.env.VITE_API_URL || import.meta.env.VITE_QR_API || 'https://qr.scutoons.com/api'
);

/** The creator app: "Open in the creator" links (`VITE_CREATOR_URL`; dev http://localhost:10000). */
export const CREATOR_URL: string = trim(
	import.meta.env.VITE_CREATOR_URL || (import.meta.env.PROD ? 'https://creator.scutoons.com' : 'http://localhost:10000')
);

/** The reader (viewer): links to toons (`VITE_VIEWER_URL`; dev http://localhost:8000). */
export const VIEWER_URL: string = trim(
	import.meta.env.VITE_VIEWER_URL || (import.meta.env.PROD ? 'https://scutoons.com' : 'http://localhost:8000')
);

/** CDN base for images, e.g. the toon avatars (`VITE_CDN_URL`; dev `python cdn/server.py 8001`). */
export const CDN_SERVER: string = trim(
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : 'http://localhost:8001')
);

/**
 * A toon's navigation avatar by convention: `<CDN>/<toonId>/avatar.webp` (the server catalog has no avatar field;
 * the viewer's toons.json names the same file).
 */
export const toonAvatarUrl = (toonId: string): string => `${CDN_SERVER}/${encodeURIComponent(toonId)}/avatar.webp`;
