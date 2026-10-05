import ports from '../../../shared/dev-ports.json' with { type: 'json' };

const trim = (url: string) => url.replace(/\/+$/, '');

/**
 * Backend API (GET /products/). `VITE_API_URL` (dev: .env.development, port in shared/dev-ports.json); prod fallback
 * https://api.scutoons.com/api. `VITE_QR_API` is the old name, still read for now.
 */
export const API_URL: string = trim(
	import.meta.env.VITE_API_URL || import.meta.env.VITE_QR_API || 'https://api.scutoons.com/api'
);

/** CDN base for product images (`VITE_CDN_URL`; dev: the local CDN, shared/dev-ports.json). */
export const CDN_SERVER: string = trim(
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : `http://localhost:${ports.cdn}`)
);

/** The main site www.scutoons.com (logo link back). `VITE_WWW_URL` (dev: shared/dev-ports.json). */
export const WWW_URL: string = trim(
	import.meta.env.VITE_WWW_URL || (import.meta.env.PROD ? 'https://scutoons.com' : `http://localhost:${ports.viewer}`)
);
