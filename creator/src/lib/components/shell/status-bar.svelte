<script lang="ts">
	import { toaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The status bar along the window's bottom, as the deck editor's (deck/kit/src/editor/status.rs): one pill – left the
	 * newest message (its level's dot, its words, its age), in the middle the open item's state as quiet chips, right the
	 * error and warning counts. A press on the message or the counts opens Problems; the panels end above it.
	 */
	type Level = 'info' | 'success' | 'warning' | 'error';
	let newest = $state<{ id: number; text: string; level: Level; at: number } | null>(null);
	$effect(() => {
		const last = toaster.items.at(-1);
		if (last && last.id !== newest?.id) newest = { id: last.id, text: last.message, level: last.type, at: Date.now() };
	});
	let now = $state(Date.now());
	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(timer);
	});
	const age = $derived.by(() => {
		if (!newest) return '';
		const min = Math.floor((now - newest.at) / 60_000);
		return min < 1 ? m['shell.ago_now']() : min < 60 ? m['shell.ago_min']({ n: min }) : m['shell.ago_h']({ n: Math.floor(min / 60) });
	});
	const doc = $derived(shell.doc);
	const errors = $derived(doc?.errors ?? 0);
	const warnings = $derived(doc?.warnings ?? 0);
	const saveWord = $derived(!doc?.save ? null : doc.saving ? m['creator.saving']() : doc.dirty ? m['shell.unsaved']() : m['creator.saved']());
	const openProblems = () => shell.open('problems');
</script>

<footer class="status-bar" data-testid="status-bar">
	<button type="button" class="message" onclick={openProblems} title={m['shell.status_open_problems']()} data-testid="status-message">
		<span class="dot {newest?.level ?? 'none'}" aria-hidden="true"></span>
		<span class="words">{newest?.text ?? m['shell.status_none']()}</span>
		{#if newest}<span class="age">{age}</span>{/if}
	</button>
	<span class="chips">
		{#if saveWord}
			<button type="button" class="chip" class:dirty={doc?.dirty} disabled={doc?.saving} onclick={() => doc?.save?.()} title={m['creator.save_draft']()} data-testid="save-state">
				<span class="dot" class:warning={doc?.dirty} class:success={!doc?.dirty} aria-hidden="true"></span>{saveWord}
			</button>
		{/if}
		{#if doc?.unpublished}<span class="chip" data-testid="status-not-live">{m['shell.not_live']()}</span>{/if}
	</span>
	{#if doc && (doc.errors !== undefined || doc.warnings !== undefined)}
		<button type="button" class="counts" onclick={openProblems} title={m['shell.status_open_problems']()} data-testid="status-counts">
			<span class:bad={errors > 0}>{m['creator.problems_errors']({ count: errors })}</span>
			<span class:warn={warnings > 0}>{m['creator.problems_warnings']({ count: warnings })}</span>
		</button>
	{/if}
</footer>

<style>
	.status-bar {
		position: fixed;
		z-index: 45;
		left: 1rem;
		right: 1rem;
		bottom: 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		box-sizing: border-box;
		height: var(--status-height);
		padding: 0 0.25rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-panel-ground);
		color: var(--editor-text);
		text-shadow: var(--editor-halo);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
	}
	button {
		display: inline-flex;
		align-items: center;
		gap: 0.45rem;
		min-height: calc(var(--status-height) - 0.4rem); /* control-ok: the status bar is one pill, its parts sit inside it (deck status.rs) */
		padding: 0 0.6rem;
		border: 0;
		border-radius: 1rem;
		background: transparent;
		color: inherit;
		font: inherit;
		text-shadow: inherit;
	}
	button:hover:not(:disabled) {
		background: var(--editor-hover);
	}
	.message {
		flex: 1 1 auto;
		min-width: 0;
		justify-content: flex-start;
	}
	.words {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.age {
		flex: none;
		color: var(--editor-muted);
	}
	.dot {
		flex: none;
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--editor-muted);
	}
	.dot.none {
		background: transparent;
		box-shadow: inset 0 0 0 1px var(--editor-line);
	}
	.dot.success {
		background: var(--editor-go);
	}
	.dot.warning {
		background: var(--editor-warn);
	}
	.dot.error {
		background: var(--editor-stop);
	}
	.chips {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.3rem;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		min-height: calc(var(--status-height) - 0.4rem);
		padding: 0 0.6rem;
		border-radius: 1rem;
		background: var(--editor-control);
		color: var(--editor-muted);
	}
	.counts {
		flex: none;
		color: var(--editor-muted);
	}
	.bad {
		color: var(--editor-stop);
	}
	.warn {
		color: var(--editor-warn);
	}
</style>
