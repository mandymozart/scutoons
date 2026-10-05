import ports from '../../../shared/dev-ports.json' with { type: 'json' };

const trim = (url: string) => url.replace(/\/+$/, '');

/**
 * Backend API (accounts, admin API). `VITE_API_URL` (dev: .env.development, the API port is in
 * shared/dev-ports.json); prod fallback https://qr.scutoons.com/api until api.scutoons.com exists. `VITE_QR_API` = old name.
 */
export const API_URL: string = trim(
	import.meta.env.VITE_API_URL || import.meta.env.VITE_QR_API || 'https://qr.scutoons.com/api'
);

/** The creator app: "Open in the creator" links (`VITE_CREATOR_URL`; dev: shared/dev-ports.json). */
export const CREATOR_URL: string = trim(
	import.meta.env.VITE_CREATOR_URL || (import.meta.env.PROD ? 'https://creator.scutoons.com' : `http://localhost:${ports.creator}`)
);

/** The reader (viewer): links to toons (`VITE_VIEWER_URL`; dev: shared/dev-ports.json). */
export const VIEWER_URL: string = trim(
	import.meta.env.VITE_VIEWER_URL || (import.meta.env.PROD ? 'https://scutoons.com' : `http://localhost:${ports.viewer}`)
);

/** CDN base for images, e.g. the toon avatars (`VITE_CDN_URL`; dev: the local CDN, shared/dev-ports.json). */
export const CDN_SERVER: string = trim(
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : `http://localhost:${ports.cdn}`)
);

/**
 * A toon's navigation avatar by convention: `<CDN>/<toonId>/avatar.webp` (the server catalog has no avatar field;
 * the viewer's toons.json names the same file).
 */
export const toonAvatarUrl = (toonId: string): string => `${CDN_SERVER}/${encodeURIComponent(toonId)}/avatar.webp`;

/**
 * The Ask service (tools/mcp, CUTOVER 10.28): the agent chat (`VITE_ASK_URL`; dev: shared/dev-ports.json "ask").
 * Production host: bunny Magic Containers (owner 2026-10-04) – set VITE_ASK_URL when it is deployed.
 */
export const ASK_URL: string = trim(import.meta.env.VITE_ASK_URL || (import.meta.env.PROD ? '' : `http://localhost:${ports.ask}/api/ask`));
