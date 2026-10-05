// @scutoons/tokens/vite: serve the web fonts of fonts.css from the CDN (agents/CUTOVER.md 10.1c, agents/ASSETS.md "Fonts").
//
// The CDN keeps one copy of shared/tokens/fonts/ under `scutoons/fonts/` (same file names, uploaded by the owner with
// assets:sync). Every scutoons.com app then shares that one cached copy: the browser's HTTP cache is keyed by the
// top-level site, and all apps are scutoons.com. Each @font-face `src` becomes
//
//   src: url(<VITE_CDN_URL>/scutoons/fonts/<file>) format('woff2'), url(./fonts/<file>) format('woff2');
//
// – the CDN first, the app's own bundled copy as the fallback: a browser that cannot load a source (404, CORS, CDN
// down) moves on to the next one. So a build never shows the system font because a file is missing on the CDN.
// The CDN base comes from VITE_CDN_URL (the same variable as every other media URL); unset = fonts.css unchanged.
//
//   import { cdnFonts } from '@scutoons/tokens/vite';   plugins: [cdnFonts(), sveltekit(), …]
import { readdirSync, readFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

/** the CDN folder of the fonts (AssetUsages::CODE_REFERENCES protects it from deletes in the asset manager) */
export const CDN_FONTS_FOLDER = 'scutoons/fonts/';

/** @param {string} path */
const slashes = (path) => path.split('\\').join('/');
const tokens = slashes(dirname(fileURLToPath(import.meta.url)));
const files = new Set(readdirSync(`${tokens}/fonts`).filter((f) => f.endsWith('.woff2')));

/**
 * `src` of an @font-face with a CDN source before every local font file of @scutoons/tokens (others stay as they are).
 * @param {string} src @param {string} cdn @returns {string}
 */
export function withCdnSources(src, cdn) {
	const base = `${cdn.replace(/\/+$/, '')}/${CDN_FONTS_FOLDER}`;
	return src.replace(/url\((['"]?)((?:[^)'"]*\/)?([^/)'"]+\.woff2))\1\)(\s*format\([^)]*\))?/g, (/** @type {string} */ local, /** @type {string} */ _q, /** @type {string} */ _path, /** @type {string} */ file, format = '') =>
		files.has(file) ? `url(${base}${file})${format}, ${local}` : local);
}

/** fonts.css of @scutoons/tokens with a CDN source before each local one @param {string} cdn @returns {string} */
export function cdnFontsCss(cdn) {
	return readFileSync(`${tokens}/fonts.css`, 'utf8').replace(/(\bsrc\s*:\s*)([^;]+)/g, (/** @type {string} */ _, /** @type {string} */ prop, /** @type {string} */ src) => prop + withCdnSources(src, cdn));
}

/** the source Vite gets for fonts.css and for index.css (which imports it); null = not ours @param {string} id @param {string} cdn @returns {string | null} */
function load(id, cdn) {
	const file = slashes(id.split('?')[0]);
	if (file === `${tokens}/fonts.css`) return cdnFontsCss(cdn);
	// index.css: the @font-face rules go to the end – an @import after them would be ignored (their order does not matter)
	if (file === `${tokens}/index.css`) return `${readFileSync(file, 'utf8').replace(/@import\s+['"]\.\/fonts\.css['"];?\s*/, '')}\n${cdnFontsCss(cdn)}`;
	return null;
}

/**
 * The Vite plugin. It loads index.css / fonts.css itself: Vite turns their url()s into asset references while it
 * inlines an imported stylesheet, before any PostCSS plugin sees them, so the rewrite has to happen on the source.
 * @param {{ cdn?: string }} [options] the CDN base (default: VITE_CDN_URL of the app's env)
 * @returns {import('vite').Plugin}
 */
export function cdnFonts(options = {}) {
	let cdn = options.cdn;
	return {
		name: 'scutoons-cdn-fonts',
		enforce: 'pre',
		/** @param {import('vite').ResolvedConfig} config */
		configResolved(config) {
			cdn ??= config.env.VITE_CDN_URL || undefined;
		},
		/** @param {string} id */
		load(id) {
			return cdn ? load(id, cdn) : null;
		}
	};
}
