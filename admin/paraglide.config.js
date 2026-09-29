/**
 * Paraglide compiler options: used by the vite plugin and by `scripts/paraglide.js` (npm run check).
 * @type {import('@inlang/paraglide-js').CompilerOptions}
 */
export default {
	project: './project.inlang',
	outdir: './src/lib/paraglide',
	// UI language of the admin only; own cookie so it doesn't switch the viewer's or the creator's language
	strategy: ['cookie', 'preferredLanguage', 'baseLocale'],
	cookieName: 'SCUTOONS_ADMIN_LOCALE'
};
