<script lang="ts">
	// The Rust tab (RIG-EDITING.md §2.3): the file tabs with Save and Compile at the right (Save a lone round icon, Compile
	// the one primary, last), the editor filling the height, Problems and Console in a band under it. Disabled actions say
	// why (their title and the muted line): over a limit, compiling, or the user may not compile (the API's 403
	// `creator_only` is a sentence, not an error).
	import { Button, IconButton } from '@scutoons/ui/editor';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { m } from '../messages.ts';
	import CodeEditor from './code-editor.svelte';
	import FileStrip from './file-strip.svelte';
	import ProblemsConsole from './problems-console.svelte';

	let { designer }: { designer: RigDesigner } = $props();

	function notice(text: string) {
		designer.notice = text;
	}
	const reason = $derived(designer.compileReason);
	// a built factory module is read-only: Compile's hover says so, the line stays empty (RULES H14)
	const line = $derived(reason === m['designer.reason_factory']() ? '' : reason);
</script>

<div class="rust" data-testid="designer-rust">
	<div class="top">
		<FileStrip {designer} onnotice={notice} />
		<div class="actions">
			<span class="state" data-testid="designer-state">
				{#if line && !designer.compiling}<span title={line}>{line}</span>{:else if designer.factoryNote}<span title={designer.factoryNote}>{designer.factoryNote}</span>{:else if designer.dirty}{m['designer.unsaved']()}{/if}
			</span>
			<IconButton alone icon="save" label={m['designer.save']()} title={m['designer.save_title']()} disabled={!designer.canSave} onclick={() => designer.save()} data-testid="designer-save" />
			<Button variant="primary" icon="cpu" disabled={!designer.canCompile} title={reason || m['designer.compile_title']()} aria-busy={designer.compiling} onclick={() => designer.compile()} data-testid="designer-compile">{m['designer.compile']()}</Button>
		</div>
	</div>
	{#if designer.notice}<p class="notice" role="status">{designer.notice}</p>{/if}
	<div class="slot"><CodeEditor {designer} /></div>
	<ProblemsConsole {designer} />
</div>

<style>
	.rust {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		color: var(--tool-text);
		/* the code and the Console's lines: the mono face, a size under the UI's */
		--designer-code-size: var(--type-caption-size);
		--designer-ui-size: var(--type-caption-size);
		/* how much of CodeMirror's own code colours shows: by night they are lifted toward the text colour */
		--code-hue: 45%;
	}
	:global([data-theme='day']) .rust {
		--code-hue: 100%;
	}
	.slot {
		flex: 1;
		min-height: 0;
		display: grid;
	}
	.top {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		min-width: 0;
		padding: var(--space-05) var(--space-075);
		border-bottom: 1px solid var(--tool-line);
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
	}
	.state {
		min-width: 0;
		max-width: 22rem;
		margin-right: auto;
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.notice {
		flex: none;
		margin: 0;
		padding: var(--space-025) var(--space-075);
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
		border-bottom: 1px solid var(--tool-line);
	}
</style>
