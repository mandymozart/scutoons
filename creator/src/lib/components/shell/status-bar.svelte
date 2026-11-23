<script lang="ts">
	import { Button, Pill } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { log } from '$lib/log/log.svelte.ts';

	/**
	 * The status bar along the window's bottom, as the deck editor's (deck/kit/src/editor/status.rs): one kit Pill (`status`) – left the
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

<Pill as="footer" status class="status-bar" data-testid="status-bar" data-control-exempt="the status pill's parts: deck-kit status.rs's height">
	<Button variant="quiet" fill onclick={openLog} title={m['shell.status_open_log']()} data-testid="status-message">
		<span class="dot {newest?.level ?? 'none'}" aria-hidden="true"></span>
		<span class="words">{newest?.text ?? m['shell.status_none']()}</span>
		{#if newest}<span class="age">{newest.age}</span>{/if}
	</Button>
	{#if saving}<span class="saving" data-testid="save-state"><span class="dot warn" aria-hidden="true"></span>{m['creator.saving']()}</span>{/if}
	{#if counts && (doc?.errors !== undefined || counts.errors || counts.warnings)}
		<Button variant="quiet" onclick={openLog} title={m['shell.status_open_log']()} data-testid="status-counts">
			<span class:bad={counts.errors > 0}>{m['creator.problems_errors']({ count: counts.errors })}</span>
			<span class:warn={counts.warnings > 0}>{m['creator.problems_warnings']({ count: counts.warnings })}</span>
		</Button>
	{/if}
</Pill>

<style>
	/* along the window's bottom, a margin in from its edges (status.rs); the panels end above it */
	:global(.kit-pill.status-bar) {
		position: fixed;
		z-index: 45;
		left: var(--kit-margin);
		right: var(--kit-margin);
		bottom: var(--kit-margin);
		--kit-icon-gap: calc(var(--editor-pad) * 0.8);
	}
	.words {
		min-width: 0;
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
		background: var(--kit-status-none-mark);
	}
	.dot.none {
		background: transparent;
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-status-none-mark);
	}
	.dot.ok {
		background: var(--kit-status-ok-mark);
	}
	.dot.info {
		background: var(--kit-status-info-mark);
	}
	.dot.warn {
		background: var(--kit-status-warning-mark);
	}
	.dot.error {
		background: var(--kit-status-error-mark);
	}
	/* a short state word in the middle: on the control ground, the pill's inside high and round */
	.saving {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: var(--kit-icon-gap);
		height: var(--kit-h);
		padding: 0 var(--editor-pad);
		border-radius: var(--kit-r);
		background: var(--tool-control);
		color: var(--tool-muted);
	}
	.bad {
		color: var(--tool-stop);
	}
	.warn {
		color: var(--tool-warn);
	}
</style>
