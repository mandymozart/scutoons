<script lang="ts">
	// The Rust tab (RIG-EDITING.md §2.3): the file tabs with Save and Compile at the right (icon-only, round; Compile primary
	// and last, one primary only), the editor filling the height, Problems and Console in a band under it. On a phone Save / Compile
	// move to a bottom bar. Disabled actions say why (their title and the muted line): over a limit, compiling, or the
	// user may not compile (the API's 403 `creator_only` is a sentence, not an error).
	import { onMount } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import type { RigDesigner } from '../designer.svelte.ts';
	import { m } from '../messages.ts';
	import CodeEditor from './code-editor.svelte';
	import FileStrip from './file-strip.svelte';
	import ProblemsConsole from './problems-console.svelte';

	let { designer }: { designer: RigDesigner } = $props();

	let phone = $state(false);
	onMount(() => {
		const query = matchMedia('(max-width: 767px)');
		const sync = () => (phone = query.matches);
		sync();
		query.addEventListener('change', sync);
		return () => query.removeEventListener('change', sync);
	});

	function notice(text: string) {
		designer.notice = text;
	}
	const reason = $derived(designer.compileReason);
</script>

{#snippet actions()}
	<span class="state" data-testid="designer-state">
		{#if reason && !designer.compiling}<span class="why" title={reason}>{reason}</span>{:else if designer.factoryNote}<span class="why" title={designer.factoryNote}>{designer.factoryNote}</span>{:else if designer.dirty}{m['designer.unsaved']()}{/if}
	</span>
	<button type="button" class="sc-round" disabled={!designer.canSave} title={m['designer.save_title']()} aria-label={m['designer.save']()} onclick={() => designer.save()} data-testid="designer-save">
		<Icon name="save" size="1rem" />
	</button>
	<button type="button" class="sc-round compile" disabled={!designer.canCompile} title={reason || m['designer.compile_title']()} aria-label={m['designer.compile']()} aria-busy={designer.compiling} onclick={() => designer.compile()} data-testid="designer-compile">
		<Icon name="cpu" size="1rem" />
	</button>
{/snippet}

<div class="rust" data-testid="designer-rust">
	<div class="top">
		<FileStrip {designer} onnotice={notice} />
		{#if !phone}<div class="actions">{@render actions()}</div>{/if}
	</div>
	{#if designer.notice}<p class="notice" role="status">{designer.notice}</p>{/if}
	<div class="slot"><CodeEditor {designer} /></div>
	<ProblemsConsole {designer} />
	{#if phone}<div class="bar">{@render actions()}</div>{/if}
</div>

<style>
	.rust {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		/* the viewer's root is 14px on a phone and 24px on a desktop: the code and the dense lines scale down there */
		--designer-code-size: 0.8125rem;
		--designer-ui-size: 0.75rem;
	}
	@media (min-width: 768px) {
		.rust {
			--designer-code-size: 0.6875rem;
			--designer-ui-size: 0.625rem;
		}
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
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.actions,
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
	}
	.bar {
		flex: none;
		justify-content: flex-end;
		padding: var(--space-05) var(--space-075) calc(var(--space-05) + env(safe-area-inset-bottom));
		border-top: var(--border-width-regular) solid var(--ink);
	}
	.state {
		min-width: 0;
		font-size: var(--designer-ui-size);
		color: var(--text-muted);
		text-transform: uppercase;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		margin-right: auto;
	}
	.bar .state {
		flex: 1;
	}
	.actions .state {
		max-width: 22rem;
	}
	/* Compile is the one primary (ink, paper icon): DESIGN "Round vs square" – a standalone icon-only button is round */
	.compile {
		background: var(--ink);
		color: var(--paper);
	}
	.compile:disabled {
		opacity: 0.5;
	}
	.notice {
		flex: none;
		margin: 0;
		padding: var(--space-025) var(--space-075);
		font-size: var(--designer-ui-size);
		color: var(--text-muted);
		border-bottom: 1px solid color-mix(in srgb, var(--ink) 20%, transparent);
	}
</style>
