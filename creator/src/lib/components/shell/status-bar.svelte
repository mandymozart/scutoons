<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { log } from '$lib/log/log.svelte.ts';

	/**
	 * The status bar along the window's bottom, as the deck editor's (deck/kit/src/editor/status.rs): one pill – left the
	 * Log's newest line (its level's dot, its words, its age), in the middle a short "saving…" while an autosave runs (Publish's dot says what is not live), right
	 * the open item's error and warning counts (the Log's problems in the Item scope). A press on the line or the counts
	 * opens the Log; the panels end above it.
	 */
	let now = $state(Date.now());
	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 15_000);
		return () => clearInterval(timer);
	});
	const newest = $derived(log.view({ show: 'events' }, '', now, 0).newest);
	const counts = $derived(log.item ? log.counts({ show: 'problems', scope: log.item }) : null);
	const doc = $derived(shell.doc);
	const openLog = () => shell.open('log');
	/** "saving…" stays long enough to be read (a save often takes less) */
	let saving = $state(false);
	$effect(() => {
		if (doc?.saving) {
			saving = true;
			return;
		}
		const timer = setTimeout(() => (saving = false), 700);
		return () => clearTimeout(timer);
	});
</script>

<footer class="status-bar" data-testid="status-bar">
	<button type="button" class="message" onclick={openLog} title={m['shell.status_open_log']()} data-testid="status-message">
		<span class="dot {newest?.level ?? 'none'}" aria-hidden="true"></span>
		<span class="words">{newest?.text ?? m['shell.status_none']()}</span>
		{#if newest}<span class="age">{newest.age}</span>{/if}
	</button>
	<span class="chips">
		{#if saving}<span class="chip" data-testid="save-state"><span class="dot warn" aria-hidden="true"></span>{m['creator.saving']()}</span>{/if}
	</span>
	{#if counts && (doc?.errors !== undefined || counts.errors || counts.warnings)}
		<button type="button" class="counts" onclick={openLog} title={m['shell.status_open_log']()} data-testid="status-counts">
			<span class:bad={counts.errors > 0}>{m['creator.problems_errors']({ count: counts.errors })}</span>
			<span class:warn={counts.warnings > 0}>{m['creator.problems_warnings']({ count: counts.warnings })}</span>
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
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1.2rem;
		background: var(--tool-panel);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
		font-family: var(--editor-sans);
		font-size: var(--editor-small);
	}
	/* the halo on its icons too (the deck's ink ring covers text and icons) */
	.status-bar :global(svg) {
		filter: var(--tool-icon-halo);
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
		background: var(--tool-hover);
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
		color: var(--tool-muted);
	}
	.dot {
		flex: none;
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--tool-muted);
	}
	.dot.none {
		background: transparent;
		box-shadow: inset 0 0 0 1px var(--tool-line);
	}
	.dot.ok {
		background: var(--tool-go);
	}
	.dot.info {
		background: var(--tool-accent);
	}
	.dot.warn {
		background: var(--tool-warn);
	}
	.dot.error {
		background: var(--tool-stop);
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
		background: var(--tool-control);
		color: var(--tool-muted);
	}
	.counts {
		flex: none;
		color: var(--tool-muted);
	}
	.bad {
		color: var(--tool-stop);
	}
	.warn {
		color: var(--tool-warn);
	}
</style>
