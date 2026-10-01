import ports from '../../../shared/dev-ports.json' with { type: 'json' };

const trim = (url: string) => url.replace(/\/+$/, '');

/** The SCUTOONS API, the same one every app uses (`VITE_API_URL`; dev: shared/dev-ports.json). */
export const API_URL: string = trim(
	import.meta.env.VITE_API_URL || (import.meta.env.PROD ? 'https://api.scutoons.com/api' : `http://localhost:${ports.api}/api`)
);

/** The reader (viewer): link on the "no access" screen (`VITE_VIEWER_URL`; dev: shared/dev-ports.json). */
export const VIEWER_URL: string = trim(
	import.meta.env.VITE_VIEWER_URL || (import.meta.env.PROD ? 'https://scutoons.com' : `http://localhost:${ports.viewer}`)
);
