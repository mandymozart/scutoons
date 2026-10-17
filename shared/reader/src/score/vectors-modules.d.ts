// the vector library's SVGs as text (Vite `?raw`) – score-collage.svelte inlines them to outline or fill them
declare module '*.svg?raw' {
	const svg: string;
	export default svg;
}
