<script lang="ts">
	// The CodeMirror editor of the active file. CodeMirror is loaded here, on mount, through the dynamic import in
	// editor.ts – the first time the Rust tab opens, never with the deck page. The designer's state is the source of
	// truth for the text; the editor reports edits back and shows markers / jumps the designer asks for.
	import { onMount, tick, untrack } from 'svelte';
	import { loadEditor, type EditorHandle } from '../editor.ts';
	import { markersFor } from '../diagnostics.ts';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { m } from '../messages.ts';

	let { designer }: { designer: RigDesigner } = $props();

	let parent: HTMLDivElement | undefined = $state();
	let editor: EditorHandle | null = $state(null);
	let failed = $state(false);

	onMount(() => {
		let gone = false;
		let handle: EditorHandle | null = null;
		void loadEditor().then(
			(create) => {
				if (gone || !parent) return;
				handle = create(parent, { label: m['designer.editor']({ name: designer.active }), onChange: (file, text) => designer.edit(file, text) });
				editor = handle;
			},
			() => (failed = true)
		);
		return () => {
			gone = true;
			handle?.destroy();
			editor = null;
		};
	});

	// the file shown (and its text when a load replaced it)
	$effect(() => {
		const ed = editor;
		const file = designer.active;
		const text = designer.files[file] ?? '';
		if (ed) untrack(() => ed.show(file, text));
	});

	// the compiler's places in the file shown
	$effect(() => {
		const ed = editor;
		const list = markersFor(designer.active, designer.diagnostics);
		if (ed) untrack(() => ed.markers(list));
	});

	// a click on a problem
	$effect(() => {
		const j = designer.jump;
		const ed = editor;
		if (!j || !ed) return;
		void tick().then(() => ed.goTo(j.line, j.column));
	});
</script>

<div class="editor" bind:this={parent} data-testid="designer-editor">
	{#if !editor}<p class="note">{failed ? m['designer.editor_failed']() : m['designer.editor_loading']()}</p>{/if}
</div>

<style>
	.editor {
		position: relative;
		min-height: 0;
		min-width: 0;
		overflow: hidden;
		background: var(--paper);
	}
	.editor :global(.cm-editor) {
		position: absolute;
		inset: 0;
	}
	.note {
		margin: 0;
		padding: var(--space-1);
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
