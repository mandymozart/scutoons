import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	// single-page app: every route is rendered in the browser (session + role gate), index.html as fallback
	kit: { adapter: adapter({ fallback: 'index.html' }) }
};

export default config;
