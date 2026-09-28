export const API_BASE = '/api';
/**
 * CDN base used by `asset()` (lib/utils/cdn.ts). Override with VITE_CDN_URL;
 * default: prod https://cdn.scutoons.com, dev the local `python cdn/server.py 8001`.
 */
export const CDN_SERVER: string = (
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : 'http://localhost:8001')
).replace(/\/+$/, '');

/** qr.scutoons.com API (accounts, items, inventory, codes). No trailing slash. Dev: VITE_QR_API=http://localhost:9000/api */
export const QR_API: string = (import.meta.env.VITE_QR_API || 'https://qr.scutoons.com/api').replace(/\/+$/, '');
/** @deprecated legacy name (with trailing slash), use QR_API */
export const QRCODE_API_SERVER = QR_API + '/';

/** Hosts whose QR codes carry SCUTOONS items (`https://qr.scutoons.com/?c=CODE`). */
export const QR_HOSTS = ['qr.scutoons.com', 'localhost:9000', '127.0.0.1:9000'];

/** Cucumber-camera detection tuning (ImageNet class probability, consecutive frames, frames per second). */
export const SCAN_OBJECT_THRESHOLD = Number(import.meta.env.VITE_SCAN_OBJECT_THRESHOLD || 0.35);
export const SCAN_OBJECT_FRAMES = 3;
export const SCAN_OBJECT_FPS = 2;
