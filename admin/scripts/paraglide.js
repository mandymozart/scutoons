// Compiles the paraglide messages (src/lib/paraglide) with the same options as the vite plugin, so
// `svelte-check` works without a dev server / build having run first (fresh clone, CI). Locale modules like the dev
// server (vite.config.ts), so a check next to a running dev server doesn't swap in ~200-380 per-message files.
import { compile } from '@inlang/paraglide-js';
import options from '../paraglide.config.js';

await compile({ ...options, outputStructure: 'locale-modules' });
