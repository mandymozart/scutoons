const trim = (url: string) => url.replace(/\/+$/, '');

/**
 * Backend API (GET /products/). `VITE_API_URL` (dev http://localhost:9000/api); prod fallback
 * https://qr.scutoons.com/api until api.scutoons.com exists. `VITE_QR_API` is the old name, still read for now.
 */
export const API_URL: string = trim(
	import.meta.env.VITE_API_URL || import.meta.env.VITE_QR_API || 'https://qr.scutoons.com/api'
);

/** CDN base for product images (`VITE_CDN_URL`; dev `python cdn/server.py 8001`). */
export const CDN_SERVER: string = trim(
	import.meta.env.VITE_CDN_URL || (import.meta.env.PROD ? 'https://cdn.scutoons.com' : 'http://localhost:8001')
);

/** The main site www.scutoons.com (logo link back). `VITE_WWW_URL` (dev http://localhost:8000). */
export const WWW_URL: string = trim(
	import.meta.env.VITE_WWW_URL || (import.meta.env.PROD ? 'https://scutoons.com' : 'http://localhost:8000')
);
