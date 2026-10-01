/**
 * Paraglide compiler options: used by the vite plugin and by `scripts/paraglide.js` (npm run check).
 * @type {import('@inlang/paraglide-js').CompilerOptions}
 */
export default {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	// UI language of the Embassy only; own cookie so it doesn't switch the other apps' language
	strategy: ['cookie', 'preferredLanguage', 'baseLocale'],
	cookieName: 'SCUTOONS_EMBASSY_LOCALE'
};
