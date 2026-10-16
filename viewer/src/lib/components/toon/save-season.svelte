<script lang="ts">
	/**
	 * "Save the season to the device" beside a season's heading on the toon page (PWA phase 3, agents/research/PWA.md §2
	 * "Phase 3", FEATURES 10.74, owner 2026-10-16 "alle Pakete angehen"): the same round button as a chapter's row
	 * (save-chapter.svelte) – ↓ saves every chapter of the season the reader may read and hasn't saved (the size first),
	 * a ring while it saves (a tap stops it; the saved chapters stay), ✓ when all of them are saved (a tap removes them).
	 * lib/offline/save-season.svelte.ts does the work; each chapter's row shows its own ring.
	 */
	import { Icon, confirmDialog } from '@scutoons/ui';
	import type { LoadedSequenceItem } from '@scutoons/content';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { seasonSaves } from '$lib/offline/save-season.svelte.ts';
	import { accessEnd } from '$lib/offline/auto-save.svelte.ts';
	import { seasonState } from '$lib/sw/saved-update-rules.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';

	let { toonId, season, chapters }: { toonId: string; season: number; chapters: LoadedSequenceItem[] } = $props();

	const membership = getMembershipContext();
	const session = getSessionContext();

	const list = $derived(
		chapters.map((c) => ({
			number: c.number,
			title: c.title ?? '',
			label: chapterName(c.number),
			readable: isReadable(c) && (!!c.isPreview || membership.isValid)
		}))
	);
	const status = $derived(seasonState(list, (n) => !!savedChapters.get(toonId, n)));
	const job = $derived(seasonSaves.job(toonId, season));
	const sizing = $derived(!!seasonSaves.sizing[`${toonId}/${season}`]);
	const percent = $derived(job ? Math.min(99, Math.floor(seasonSaves.share(toonId, season) * 100)) : 0);
	const name = $derived(m['saved.season_name']({ number: season }));
	let asking = $state(false);
	let failed = $state(false);

	const label = $derived(
		job
			? m['saved.season_saving']({ done: job.done.length, count: job.chapters.length })
			: status === 'saved'
				? m['saved.season_saved']()
				: failed
					? m['saved.failed']()
					: m['saved.season_save']()
	);

	async function onclick() {
		if (asking || sizing) return;
		if (!job && status !== 'saved') {
			// the season's run: not held by `asking` – a tap while it runs asks to stop it (start() guards itself)
			failed = false;
			const result = await seasonSaves
				.start({
					toon: toonId,
					toonTitle: (page.data.toon as { title?: string } | undefined)?.title ?? toonId,
					season,
					chapters: list,
					locale: getLocale(),
					accessUntil: accessEnd(membership, session)
				})
				.catch(() => 'failed' as const);
			failed = result === 'failed';
			return;
		}
		asking = true;
		try {
			if (job) {
				if (
					await confirmDialog({
						title: m['saved.season_stop_title']({ season: name }),
						message: m['saved.season_stop_message'](),
						confirmLabel: m['saved.stop'](),
						cancelLabel: m['common.close']()
					})
				)
					seasonSaves.cancel(toonId, season);
			} else {
				const saved = list.filter((c) => savedChapters.get(toonId, c.number));
				if (
					await confirmDialog({
						title: m['saved.season_remove_title']({ season: name }),
						confirmLabel: m['saved.remove'](),
						cancelLabel: m['common.cancel']()
					})
				)
					await seasonSaves.remove(
						toonId,
						saved.map((c) => c.number)
					);
			}
		} catch {
			failed = true;
		} finally {
			asking = false;
		}
	}
</script>

{#if savedChapters.supported && (status !== 'none' || job)}
	<button
		type="button"
		class="save-season"
		class:saved={status === 'saved' && !job}
		class:saving={!!job}
		class:sizing
		style:--percent="{percent}%"
		title={label}
		aria-label={label}
		aria-busy={sizing}
		data-testid="save-season"
		data-state={job ? 'saving' : sizing ? 'sizing' : status === 'saved' ? 'saved' : failed ? 'failed' : 'idle'}
		{onclick}
	>
		{#if job}
			<Icon name="x" size="0.875rem" />
		{:else if status === 'saved'}
			<Icon name="check" size="1rem" />
		{:else if failed}
			<Icon name="alert-circle" size="1rem" />
		{:else}
			<Icon name="download" size="1rem" />
		{/if}
	</button>
{/if}

<style>
	/* the chapter rows' save button (save-chapter.svelte): round, at the control height */
	.save-season {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	.save-season.saved {
		background: var(--ink);
		color: var(--paper);
	}
	/* asking the sizes: the button waits */
	.save-season.sizing {
		opacity: 0.5;
		cursor: progress;
	}
	.save-season.saving {
		border-color: transparent;
		background:
			radial-gradient(closest-side, var(--paper) calc(100% - var(--border-width-regular) * 1.5), transparent calc(100% - var(--border-width-regular) * 1.5)),
			conic-gradient(var(--ink) var(--percent), var(--border-muted) 0);
	}
</style>
