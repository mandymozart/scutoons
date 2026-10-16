<script lang="ts">
	/**
	 * "Save to device" on a chapter's row (toon page + the chapter menu; PWA phase 3, agents/research/PWA.md §2
	 * "Phase 3", FEATURES 10.74): one round button – ↓ save (the size first: "about 18 MB", "without videos" when they
	 * are big, mobile data / data saver said), a ring while saving (a tap stops it), ✓ saved (a tap removes it).
	 * Only for chapters the reader may read (published; a free preview or the reader's access) and only with a service
	 * worker (offline needs it). lib/offline/saved-chapters.svelte.ts does the work.
	 */
	import { Icon, choiceDialog, confirmDialog } from '@scutoons/ui';
	import { effectiveQuality, type LoadedSequenceItem } from '@scutoons/content';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isReadable } from '$lib/draft-preview.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { accessUntilOf, dataWarnings, offerWithoutVideos, planSave, roundMb, type ConnectionHint } from '$lib/sw/saved-rules.ts';
	import { chapterName } from '$lib/utils/chapter-name.ts';

	let { toonId, chapter }: { toonId: string; chapter: LoadedSequenceItem } = $props();

	const membership = getMembershipContext();
	const session = getSessionContext();

	const saved = $derived(savedChapters.get(toonId, chapter.number));
	const job = $derived(savedChapters.job(toonId, chapter.number));
	const may = $derived(isReadable(chapter) && (!!chapter.isPreview || membership.isValid));
	const shown = $derived(savedChapters.supported && (!!saved || !!job || may));
	const percent = $derived(job ? Math.min(99, Math.floor((job.done / Math.max(1, job.total)) * 100)) : 0);
	let asking = $state(false);
	let failed = $state(false);

	const label = $derived(
		job ? m['saved.saving']({ percent }) : saved ? m['saved.saved']() : failed ? m['saved.failed']() : m['saved.save']()
	);

	async function save(name: string) {
		const locale = getLocale();
		const list = await savedChapters.files(toonId, chapter.number, locale);
		const level = effectiveQuality(mediaQuality.setting ?? 'auto', mediaQuality.auto);
		const all = planSave(list.files, level, true);
		const lean = planSave(list.files, level, false);
		const warn = dataWarnings(all.bytes, (navigator as Navigator & { connection?: ConnectionHint }).connection);
		const message = [
			m['saved.save_message']({ size: roundMb(all.bytes) }),
			warn.cellular ? m['saved.cellular']() : '',
			warn.saveData ? m['saved.data_saver']() : ''
		]
			.filter(Boolean)
			.join(' ');
		const title = m['saved.save_title']({ chapter: name });
		let videos = true;
		if (offerWithoutVideos(all)) {
			const choice = await choiceDialog({
				title,
				message,
				cancelLabel: m['common.cancel'](),
				choices: [
					{ id: 'all', label: m['saved.with_videos']({ size: roundMb(all.bytes) }) },
					{ id: 'lean', label: m['saved.without_videos']({ size: roundMb(lean.bytes) }) }
				]
			});
			if (!choice) return;
			videos = choice === 'all';
		} else if (!(await confirmDialog({ title, message, confirmLabel: m['saved.save_confirm'](), cancelLabel: m['common.cancel']() }))) return;

		// the access the chapter is saved with: the latest end of the reader's valid grants (device code, account)
		const grants = [
			...(membership.deviceValid ? [membership.device?.expires_at ?? null] : []),
			...(session.hasAccess ? [session.me?.access?.expires_at ?? null] : [])
		];
		const result = await savedChapters.save(toonId, chapter.number, {
			title: chapter.title ?? '',
			label: name,
			toonTitle: (page.data.toon as { title?: string } | undefined)?.title ?? toonId,
			locale,
			level,
			videos,
			list,
			accessUntil: accessUntilOf(grants)
		});
		failed = result === 'failed';
	}

	async function onclick(event: MouseEvent) {
		// the row opens the chapter – this button doesn't
		event.stopPropagation();
		if (asking) return;
		asking = true;
		const name = chapterName(chapter.number);
		try {
			if (job) {
				if (await confirmDialog({ title: m['saved.stop_title']({ chapter: name }), confirmLabel: m['saved.stop'](), cancelLabel: m['common.close']() }))
					savedChapters.cancel(toonId, chapter.number);
			} else if (saved) {
				if (
					await confirmDialog({
						title: m['saved.remove_title']({ chapter: name }),
						message: m['saved.remove_message']({ size: roundMb(saved.bytes) }),
						confirmLabel: m['saved.remove'](),
						cancelLabel: m['common.cancel']()
					})
				)
					await savedChapters.remove(toonId, chapter.number);
			} else {
				failed = false;
				await save(name);
			}
		} catch {
			failed = true;
		} finally {
			asking = false;
		}
	}
</script>

{#if shown}
	<button
		type="button"
		class="save-chapter"
		class:saved={!!saved && !job}
		class:saving={!!job}
		style:--percent="{percent}%"
		title={label}
		aria-label={label}
		data-testid="save-chapter"
		data-state={job ? 'saving' : saved ? 'saved' : failed ? 'failed' : 'idle'}
		{onclick}
		onkeydown={(e) => e.stopPropagation()}
	>
		{#if job}
			<Icon name="x" size="0.875rem" />
		{:else if saved}
			<Icon name="check" size="1rem" />
		{:else if failed}
			<Icon name="alert-circle" size="1rem" />
		{:else}
			<Icon name="download" size="1rem" />
		{/if}
	</button>
{:else if savedChapters.supported}
	<!-- the button's room, so the numbers of rows without one stay in line -->
	<span class="save-chapter-room" aria-hidden="true"></span>
{/if}

<style>
	/* a standalone icon-only button: round (DESIGN "Round vs square"), at the control height */
	.save-chapter {
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
	.save-chapter-room {
		flex: none;
		width: var(--control-height);
	}
	/* saved: filled */
	.save-chapter.saved {
		background: var(--ink);
		color: var(--paper);
	}
	/* saving: the ring fills with the progress */
	.save-chapter.saving {
		border-color: transparent;
		background:
			radial-gradient(closest-side, var(--paper) calc(100% - var(--border-width-regular) * 1.5), transparent calc(100% - var(--border-width-regular) * 1.5)),
			conic-gradient(var(--ink) var(--percent), var(--border-muted) 0);
	}
</style>
