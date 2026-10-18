<script lang="ts">
	// Account page → Offline-Inhalte → "Saved chapters" (PWA phase 3, agents/research/PWA.md §2 "Phase 3"; ACCOUNT-PAGE.md
	// §0 – "save by themselves" is its own setting, auto-save-setting.svelte): what is saved on this device,
	// the space it takes (and what the browser allows), one "Remove" per chapter and "Remove all". Nothing when nothing
	// is saved (saving happens at the season's heading on the toon page, or by itself). The running saves and refreshes with their
	// bars (a season: one bar per chapter), "Updated …" after a new version came in, "Update" when one waits for the tap
	// (lib/offline/refresh-saved.svelte.ts), "Save the rest" of a season begun here (saved-seasons.svelte); in a Safari tab
	// the honest note that Safari deletes the saves after 7 days without a visit (not in the home-screen app).
	import { confirmDialog, Icon } from '@scutoons/ui';
	import { onMount } from 'svelte';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { roundMb, SAVE_KINDS, type SaveKind } from '$lib/sw/saved-rules.ts';
	import SaveProgress from './save-progress.svelte';
	import SeasonProgress from './season-progress.svelte';
	import SavedSeasons from './saved-seasons.svelte';
	import { updateSaved } from '$lib/offline/refresh-saved.svelte.ts';
	import { seasonSaves } from '$lib/offline/save-season.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { install } from '$lib/install.svelte.ts';
	import { isSafari } from '@scutoons/pwa';

	/** Safari outside the home-screen app: its saves go after 7 days without a visit – said honestly */
	const safariTab = $derived(isSafari(navigator.userAgent, navigator.maxTouchPoints) && !install.installed);
	let estimate = $state<{ usage: number; quota: number } | null>(null);
	const count = $derived(savedChapters.list.length);
	const running = $derived(Object.entries(savedChapters.jobs));
	const seasons = $derived(Object.values(seasonSaves.jobs));
	/** "16.10.2026" in the reader's language */
	const day = (at: number) => new Date(at).toLocaleDateString(getLocale());
	// a finished save: the list and the space again
	let wasRunning = 0;
	$effect(() => {
		const now = running.length;
		if (now < wasRunning) void measure();
		wasRunning = now;
	});

	const KIND_WORD: Record<SaveKind, () => string> = {
		image: () => m['saved.kind_image'](),
		audio: () => m['saved.kind_audio'](),
		video: () => m['saved.kind_video'](),
		data: () => m['saved.kind_data']()
	};
	/** what a saved chapter is made of: "Bilder 12 MB · Musik & Klänge 4 MB …" */
	const breakdown = (kinds: Record<SaveKind, number>) =>
		SAVE_KINDS.filter((k) => kinds[k] > 0)
			.map((k) => `${KIND_WORD[k]()} ${roundMb(kinds[k])} MB`)
			.join(' · ');

	async function measure() {
		const e = await navigator.storage?.estimate?.().catch(() => undefined);
		estimate = e?.usage !== undefined && e.quota ? { usage: e.usage, quota: e.quota } : null;
	}
	onMount(() => {
		void savedChapters.refresh().then(measure);
	});

	async function remove(toon: string, chapter: number, title: string) {
		if (!(await confirmDialog({ title: m['saved.remove_title']({ chapter: title }), confirmLabel: m['saved.remove'](), cancelLabel: m['common.cancel']() }))) return;
		await savedChapters.remove(toon, chapter);
		await measure();
	}

	async function removeAll() {
		if (!(await confirmDialog({ title: m['saved.remove_all_title'](), confirmLabel: m['saved.remove_all'](), cancelLabel: m['common.cancel']() }))) return;
		await savedChapters.removeAll();
		await measure();
	}
</script>

{#if count || running.length || seasons.length}
	<fieldset class="saved-chapters" data-testid="saved-chapters-setting">
		<legend>{m['saved.title']()}</legend>
		<!-- the saves running now: a bar for the chapter and one per kind (owner 2026-10-16) -->
		{#each seasons as job (job.toon + '/' + job.season)}
			<SeasonProgress {job} />
		{/each}
		{#each running as [id, job] (id)}
			<SaveProgress {job} onstop={() => job.controller.abort()} />
		{/each}
		<ul>
			{#each savedChapters.list as saved (saved.toon + '/' + saved.chapter)}
				{@const name = `${saved.toonTitle ?? saved.toon} · ${saved.label ?? m['common.chapter']({ number: saved.chapter })}`}
				{@const id = `${saved.toon}/${saved.chapter}`}
				{@const waiting = savedChapters.outdated[id]}
				<li data-testid="saved-chapter" data-id={id}>
					<span class="entry">
						<a href={localizeHref(`/${saved.toon}/chapter/${saved.chapter}`)}>{name}{saved.title ? ` · ${saved.title}` : ''}</a>
						{#if saved.kinds}<small class="kinds">{breakdown(saved.kinds)}</small>{/if}
						{#if savedChapters.jobs[id]?.mode === 'refresh'}
							<small data-testid="saved-updating">{m['saved.updating']()}</small>
						{:else if waiting !== undefined}
							<small class="update">
								{m['saved.outdated']()}
								<button type="button" class="link" onclick={() => updateSaved(saved.toon, saved.chapter)} data-testid="saved-update">{m['saved.update']({ size: roundMb(waiting) })}</button>
							</small>
						{:else if saved.refreshedAt}
							<small data-testid="saved-updated">{m['saved.updated']({ date: day(saved.refreshedAt) })}</small>
						{/if}
					</span>
					<small>{roundMb(saved.bytes)} MB</small>
					<button type="button" class="round" title={m['saved.remove']()} aria-label={m['saved.remove']()} onclick={() => remove(saved.toon, saved.chapter, name)}>
						<Icon name="trash-2" size="1rem" />
					</button>
				</li>
			{/each}
		</ul>
		<SavedSeasons />
		{#if count}<p>
			<button type="button" class="btn-secondary" onclick={removeAll} data-testid="saved-remove-all">{m['saved.remove_all']()}</button>
			<small>
				{m['saved.summary']({ count, size: roundMb(savedChapters.bytes) })}
				{#if estimate}· {m['saved.storage']({ used: roundMb(estimate.usage), quota: roundMb(estimate.quota) })}{/if}
			</small>
		</p>{/if}
		{#if count && safariTab}
			<!-- WebKit deletes a site's storage after 7 days without a visit – not for a home-screen app (PWA.md §4) -->
			<p class="safari" data-testid="saved-safari-hint"><small>{m['saved.safari_7_days']()}</small></p>
		{/if}
	</fieldset>
{/if}

<style>
	.saved-chapters {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
	}
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	ul {
		list-style: none;
		margin: 0 0 0.75rem;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.25rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.entry {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	li a {
		color: inherit;
	}
	/* the size beside the entry: "< 1 MB" on one line (it broke after "<" on a phone) */
	li > small {
		flex: none;
		white-space: nowrap;
	}
	p {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	.btn-secondary {
		height: var(--control-height);
		padding: 0 1rem;
	}
	/* a standalone icon-only button: round */
	.round {
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
	/* "Update · about 2 MB": a text button in the line (the entry's own action, not a primary one) */
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
