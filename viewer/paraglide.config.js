/**
 * Paraglide compiler options: used by the vite plugin and by `scripts/paraglide.js` (npm run check).
 *
 * Locale rule (agents/MEMORY.md 2026-09-30 "viewer locale"):
 * - an explicit prefix `/en/…` or `/de/…` always wins (route strategy: url first; the visit also writes the cookie, so
 *   unprefixed links followed from there stay in that language);
 * - an unprefixed URL (every printed / QR URL) uses the cookie `PARAGLIDE_LOCALE`, then the browser language, then `de`.
 * URL patterns stay paraglide's default (`de` unprefixed, `en` under `/en/`, `/de/` understood), so `localizeHref`
 * keeps German links unprefixed and English links under `/en/`. The default pattern alone can't be the global `url`
 * strategy: it reads every unprefixed path as the base locale `de` and the cookie would never be consulted.
 * @type {import('@inlang/paraglide-js').CompilerOptions}
 */
export default {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	strategy: ['cookie', 'preferredLanguage', 'baseLocale'],
	routeStrategies: [{ match: '/:locale(de|en)/:path(.*)?', strategy: ['url', 'cookie', 'baseLocale'] }]
};
