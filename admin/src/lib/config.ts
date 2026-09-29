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
