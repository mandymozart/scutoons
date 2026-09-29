/**
 * Paraglide compiler options: used by the vite plugin and by `scripts/paraglide.js` (npm run check).
 * Locale from the URL prefix first (`/de/`, `/en/` – the viewer's `/shop` redirect sends the reader's locale), then
 * the cookie, then the browser language.
 * @type {import('@inlang/paraglide-js').CompilerOptions}
 */
export default {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	strategy: ['url', 'cookie', 'preferredLanguage', 'baseLocale'],
	urlPatterns: [
		{
			pattern: '/:path(.*)?',
			localized: [
				['de', '/de/:path(.*)?'],
				['en', '/en/:path(.*)?']
			]
		}
	]
};
