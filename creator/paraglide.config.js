/**
 * Paraglide compiler options: used by the vite plugin and by `scripts/paraglide.js` (npm run check).
 * @type {import('@inlang/paraglide-js').CompilerOptions}
 */
export default {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	// UI language of the editor only (content locales are edited side by side); own cookie so it doesn't switch the
	// viewer's language on the same host
	strategy: ['cookie', 'preferredLanguage', 'baseLocale'],
	cookieName: 'SCUTOONS_CREATOR_LOCALE'
};
