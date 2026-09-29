export const API_BASE = '/api';
/**
 * CDN base used by `asset()` (lib/utils/cdn.ts). Override with VITE_CDN_URL;
 * default: prod https://cdn.scutoons.com, dev the local `python cdn/server.py 8001`.
 */
export const CDN_SERVER: string = (
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : 'http://localhost:8001')
).replace(/\/+$/, '');

/**
 * Backend API (accounts, access, items, inventory, content store). No trailing slash. `VITE_API_URL`
 * (dev http://localhost:9000/api); prod fallback https://qr.scutoons.com/api until api.scutoons.com exists.
 * `VITE_QR_API` is the old name, still read for now.
 */
export const API_URL: string = (
	import.meta.env.VITE_API_URL ||
	import.meta.env.VITE_QR_API ||
	'https://qr.scutoons.com/api'
).replace(/\/+$/, '');

/** Hosts whose QR codes carry SCUTOONS items (`https://qr.scutoons.com/?c=CODE`). */
export const QR_HOSTS = ['qr.scutoons.com', 'localhost:9000', '127.0.0.1:9000'];

/** Cucumber-camera detection tuning (ImageNet class probability, consecutive frames, frames per second). */
export const SCAN_OBJECT_THRESHOLD = Number(import.meta.env.VITE_SCAN_OBJECT_THRESHOLD || 0.35);
export const SCAN_OBJECT_FRAMES = 3;
export const SCAN_OBJECT_FPS = 2;

/** The creator app (own origin), linked from the account page for the creator role. `VITE_CREATOR_URL`. */
export const CREATOR_URL: string = (
	import.meta.env.VITE_CREATOR_URL || (import.meta.env.PROD ? 'https://creator.scutoons.com' : 'http://localhost:10000')
).replace(/\/+$/, '');

/** The shop (own app, shop.scutoons.com): menu links, "Buy access", `/shop` redirect. `VITE_SHOP_URL`. */
export const SHOP_URL: string = (
	import.meta.env.VITE_SHOP_URL || (import.meta.env.PROD ? 'https://shop.scutoons.com' : 'http://localhost:12000')
).replace(/\/+$/, '');

/** The shop in a locale: `<SHOP_URL>/<locale>/` (the shop reads the locale from the URL prefix). */
export function shopUrl(locale: string): string {
	return `${SHOP_URL}/${locale}/`;
}
