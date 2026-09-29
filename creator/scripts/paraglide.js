// Compiles the paraglide messages (src/lib/paraglide) with the same options as the vite plugin, so
// `svelte-check` works without a dev server / build having run first (fresh clone, CI).
import { compile } from '@inlang/paraglide-js';
import options from '../paraglide.config.js';

await compile(options);
