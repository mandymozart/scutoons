<script>
	// The main view's editor switch (agents/research/VARIANTS-CANVAS.md §3.1, STUDIO-LAYOUT §4 C – Blender's editor type,
	// only for the main): a square Segmented "Viewport · Variants" at the start of whichever tool row the main shows (the 3D
	// view's, a clip's, the trim pair's, the board's); key V outside text fields (App). Each step remembers its editor
	// (steps.svelte.js) – Generate opens Variants; Play goes back to the Viewport. Where the 3D view's tool row is too
	// narrow for the words beside its own tools (the Script step at 1440: its controls would wrap to a second line and
	// take the view's height), the switch keeps only its icons – the words stay its names and tooltips (the container
	// `viewport-tools`, Viewport.svelte).
	import { Segmented } from '@scutoons/ui';
	import { EDITORS, here, setEditor } from '../steps.svelte.js';
</script>

<div class="editor-switch">
	<Segmented label="The main view (V)" radio options={EDITORS} value={here().editor} onchange={setEditor} testid="editor-switch" />
</div>

<style>
	.editor-switch {
		display: contents;
	}
	/* the 3D view's tools need ≈ 46rem with the words (the switch, Move · Turn, Texts, Shot camera) */
	@container viewport-tools (max-width: 46rem) {
		.editor-switch :global(button) {
			font-size: 0;
			gap: 0;
		}
		.editor-switch :global(svg) {
			width: 1.25rem;
			height: 1.25rem;
		}
	}
</style>
