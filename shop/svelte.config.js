import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	// single-page app: the catalog comes from the API at runtime (/api/products/), index.html as fallback
	kit: { adapter: adapter({ fallback: 'index.html' }) }
};

export default config;
