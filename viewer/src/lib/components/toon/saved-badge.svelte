<script lang="ts">
	/**
	 * "Saved on this device" after a chapter's name in the chapter list (toon page + the chapter menu; PWA phase 3,
	 * agents/research/PWA.md §2 "Phase 3", owner 2026-10-17): a small muted mark in the look of the creator's slice marks
	 * (creator chapter/marks.svelte) – ✓ in a circle when the chapter is saved here, a tiny ring filling while it is being
	 * saved or updated. Not a control: saving starts at the season's heading (save-season.svelte), by itself
	 * (auto-save) or on the account's offline page, which also removes. Nothing without a service worker.
	 */
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';

	let { toonId, chapter }: { toonId: string; chapter: number } = $props();

	const saved = $derived(savedChapters.get(toonId, chapter));
	const job = $derived(savedChapters.job(toonId, chapter));
	const percent = $derived(job ? Math.min(99, Math.floor((job.done / Math.max(1, job.total)) * 100)) : 0);
	const label = $derived(
		job ? (job.mode === 'refresh' ? m['saved.refreshing']({ percent }) : m['saved.saving']({ percent })) : m['saved.saved']()
	);
</script>

{#if savedChapters.supported && (saved || job)}
	<span
		class="saved-badge"
		role="img"
		aria-label={label}
		title={label}
		data-testid="saved-badge"
		data-state={job ? 'saving' : 'saved'}
	>
		{#if job}
			<span class="ring" style:--percent="{percent}%"></span>
		{:else}
			<Icon name="check-circle" size="0.875rem" />
		{/if}
	</span>
{/if}

<style>
	/* the creator's marks: caption size, muted, in the text line */
	.saved-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 0.875rem;
		height: 0.875rem;
		margin-left: 0.375rem;
		vertical-align: -0.125em;
		color: var(--text-muted);
	}
	/* a ring filling with the progress (the track muted, the saved part in the muted ink) */
	.ring {
		display: block;
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 50%;
		background: conic-gradient(currentColor var(--percent), var(--border-muted) 0);
		/* the hole cut out (not painted over), so the ring sits on any background (the dark chapter menu too) */
		--hole: radial-gradient(closest-side, transparent calc(100% - var(--border-width-regular) * 1.25), #000 calc(100% - var(--border-width-regular) * 1.25));
		-webkit-mask: var(--hole);
		mask: var(--hole);
	}
</style>
