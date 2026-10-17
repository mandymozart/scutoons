// the vector library's SVGs as text (Vite `?raw`, score-collage.svelte inlines them to outline or fill them) or as a
// URL (Vite `?url`, score-illumination.svelte's masks)
declare module '*.svg?raw' {
	const svg: string;
	export default svg;
}

declare module '*.svg?url' {
	const url: string;
	export default url;
}
