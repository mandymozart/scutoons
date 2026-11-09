import ports from '../../../shared/dev-ports.json' with { type: 'json' };

/**
 * CDN base used by `asset()` (lib/utils/cdn.ts). Override with VITE_CDN_URL;
 * default: prod https://cdn.scutoons.com, dev the local CDN (shared/dev-ports.json).
 */
export const CDN_SERVER: string = (
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : `http://localhost:${ports.cdn}`)
).replace(/\/+$/, '');

/**
 * Backend API (accounts, access, items, inventory, content store). No trailing slash. `VITE_API_URL`
 * (dev: .env.development, port in shared/dev-ports.json); prod fallback https://api.scutoons.com/api.
 * `VITE_QR_API` is the old name, still read for now.
 */
export const API_URL: string = (
	import.meta.env.VITE_API_URL ||
	import.meta.env.VITE_QR_API ||
	'https://api.scutoons.com/api'
).replace(/\/+$/, '');

/**
 * The hosted Live relay (#199, LIVE.md B.4: bunny Magic Containers `scutoons-live`), no trailing slash: /live/deck joins
 * `<LIVE_URL>/live/<code>` unless `?relay=` names another. `VITE_LIVE_URL` – the same in dev (a local relay: `?relay=`).
 */
export const LIVE_URL: string = (import.meta.env.VITE_LIVE_URL || 'wss://live.scutoons.com').replace(/\/+$/, '');
/** #438: a QUIC door under another name than the relay's (`host:port`, `VITE_LIVE_QUIC`); unset: the relay's own (#441) */
export const LIVE_QUIC: string | undefined = import.meta.env.VITE_LIVE_QUIC || undefined;

/** Hosts whose QR codes carry SCUTOONS items (`https://qr.scutoons.com/?c=CODE`). */
export const QR_HOSTS = ['qr.scutoons.com', `localhost:${ports.api}`, `127.0.0.1:${ports.api}`];

/**
 * Object-camera tuning (`scoreScan` / `scanPasses` in @scutoons/content): a frame passes at a summed ImageNet probability
 * of THRESHOLD, or when the object is among the TOP predictions with at least MIN; FRAMES passing frames in a row grant
 * the item; FPS frames per second. Owner phone test 2026-10-02: a real cucumber scored 0.10–0.16 at rank 2–3, under the aim square up to 0.45; lowered
 * the same day so aiming takes less long.
 */
export const SCAN_OBJECT_THRESHOLD = Number(import.meta.env.VITE_SCAN_OBJECT_THRESHOLD || 0.2);
export const SCAN_OBJECT_TOP = 3;
export const SCAN_OBJECT_MIN = 0.08;
/** passing frames in a row (owner 2026-10-02: "so we don't have to aim so long" – 2 frames = 1 s) */
export const SCAN_OBJECT_FRAMES = 2;
export const SCAN_OBJECT_FPS = 2;

/** The creator app (own origin), linked from the account page for the creator role. `VITE_CREATOR_URL`. */
export const CREATOR_URL: string = (
	import.meta.env.VITE_CREATOR_URL || (import.meta.env.PROD ? 'https://creator.scutoons.com' : `http://localhost:${ports.creator}`)
).replace(/\/+$/, '');

/** The shop (own app, shop.scutoons.com): menu links, "Buy access", `/shop` redirect. `VITE_SHOP_URL`. */
export const SHOP_URL: string = (
	import.meta.env.VITE_SHOP_URL || (import.meta.env.PROD ? 'https://shop.scutoons.com' : `http://localhost:${ports.shop}`)
).replace(/\/+$/, '');

/** The shop in a locale: `<SHOP_URL>/<locale>/` (the shop reads the locale from the URL prefix). */
export function shopUrl(locale: string): string {
	return `${SHOP_URL}/${locale}/`;
}
