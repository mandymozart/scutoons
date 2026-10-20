<script lang="ts">
	// The main view's editor switch (#125; VARIANTS-CANVAS §3.1, STUDIO-LAYOUT §4 C – Blender's editor type, only for the
	// main): a square Segmented "Viewport · Variants" at the start of whichever tool row the main shows (the 3D view's,
	// the board's); key V outside text fields (main-view). Each step remembers its editor – Generate opens Variants.
	import { Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import type { Editor } from '$lib/editor/storyboard-board.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();
	const options = $derived([
		{ value: 'viewport' as Editor, icon: 'box', label: m['storyboard.editor_viewport'](), title: m['storyboard.editor_viewport_hint']() },
		{ value: 'variants' as Editor, icon: 'grid', label: m['storyboard.editor_variants'](), title: m['storyboard.editor_variants_hint']() }
	]);
</script>

<span class="editor-switch">
	<Segmented label={m['storyboard.editor_switch']()} radio {options} value={sb.board.editor} onchange={(v) => sb.board.setEditor(v)} testid="editor-switch" />
</span>

<style>
	.editor-switch {
		display: contents;
	}
	/* a narrow main: the icons stay, the words are their names and tooltips */
	@container (max-width: 40rem) {
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
