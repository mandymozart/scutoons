<script lang="ts">
	/**
	 * "Save the season to the device" at a season's heading on the toon page (PWA phase 3, agents/research/PWA.md §2
	 * "Phase 3", FEATURES 10.74; owner 2026-10-17: an icon, not a big round button, and it must not move the heading):
	 * the offline group's download icon (groups.ts `download-cloud`) saves every chapter of the season the reader may read
	 * and hasn't saved (the size first), a small ring while it saves (a tap stops it; the saved chapters stay), ✓ when all
	 * of them are saved (a tap removes them). Placed absolutely at the heading's right end on its text line (the parent
	 * `.list-head` is `position: relative` and sets `--head-line`), the touch target by padding. Each chapter shows its
	 * own state as a badge after its name (saved-badge.svelte). lib/offline/save-season.svelte.ts does the work.
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
	import { savedWhole } from '$lib/sw/saved-rules.ts';
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
	// the season's ✓ only when every readable chapter is here whole (its videos too – owner 2026-10-17)
	const status = $derived(seasonState(list, (n) => savedWhole(savedChapters.get(toonId, n))));
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
			<span class="ring"></span>
		{:else if status === 'saved'}
			<Icon name="check-circle" size="1rem" />
		{:else if failed}
			<Icon name="alert-circle" size="1rem" />
		{:else}
			<Icon name="download-cloud" size="1rem" />
		{/if}
	</button>
{/if}

<style>
	/* an icon, not a round button (owner 2026-10-17): 1rem, on the heading's text line at its right end, out of the
	   flow so the heading doesn't move; padding makes the control-height touch target, the negative margin takes it back */
	.save-season {
		--pad: calc((var(--control-height) - 1rem) / 2);
		position: absolute;
		right: 0;
		/* the icon's centre on the centre of the heading's (last) line: the head's bottom padding + half a line */
		bottom: calc(var(--head-pad, 0.5rem) + var(--head-line, 1.5rem) / 2 - 0.5rem);
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: content-box;
		width: 1rem;
		height: 1rem;
		min-width: 0;
		min-height: 0;
		padding: var(--pad);
		margin: calc(-1 * var(--pad));
		border: 0;
		border-radius: 0;
		background: none;
		color: var(--ink);
		cursor: pointer;
	}
	/* asking the sizes: the icon waits */
	.save-season.sizing {
		opacity: 0.5;
		cursor: progress;
	}
	.save-season.saved {
		color: var(--text-muted);
	}
	/* saving: a small ring filling with the season's progress (the hole cut out, so any background shows through) */
	.ring {
		display: block;
		width: 0.875rem;
		height: 0.875rem;
		border-radius: 50%;
		background: conic-gradient(var(--ink) var(--percent), var(--border-muted) 0);
		--hole: radial-gradient(closest-side, transparent calc(100% - var(--border-width-regular) * 1.25), #000 calc(100% - var(--border-width-regular) * 1.25));
		-webkit-mask: var(--hole);
		mask: var(--hole);
	}
</style>
