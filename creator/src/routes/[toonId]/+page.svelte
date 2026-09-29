<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar } from '@scutoons/ui';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc, LOCALES, SOURCE_LOCALE } from '$lib/editor/doc.svelte.ts';
	import { newChapter } from '$lib/editor/defaults.ts';
	import { Status } from '@scutoons/content';
	import type { SequenceItem } from '@scutoons/content';
	import StyleGuide from '$lib/components/style-guide.svelte';
	import FileStatus from '$lib/components/file-status.svelte';

	const toonId = $derived(page.params.toonId as string);

	let season = $state<ContentDoc | null>(null);
	let chapters = $state<Record<number, ContentDoc>>({});
	let busy = $state(false);
	let message = $state<string | null>(null);
	let error = $state<string | null>(null);

	const sequence = $derived<SequenceItem[]>(season?.data?.[SOURCE_LOCALE]?.sequence ?? []);

	async function load() {
		const doc = new ContentDoc(toonId, 'season1.json');
		await doc.load();
		season = doc;
		const docs: Record<number, ContentDoc> = {};
		await Promise.all(
			(doc.data[SOURCE_LOCALE]?.sequence ?? []).map(async (item: SequenceItem) => {
				const chapter = new ContentDoc(toonId, `chapter${item.number}.json`);
				await chapter.load();
				docs[item.number] = chapter;
			})
		);
		chapters = docs;
	}

	onMount(load);

	async function createChapter() {
		if (!season) return;
		const number = Math.max(0, ...sequence.map((s) => s.number)) + 1;
		const author = chapters[sequence[sequence.length - 1]?.number]?.data?.[SOURCE_LOCALE]?.author ?? 'Mandy Mozart';
		busy = true;
		error = null;
		try {
			const chapter = new ContentDoc(toonId, `chapter${number}.json`);
			chapter.create((locale) => newChapter(number, locale, author));
			await chapter.save(`new chapter ${number}`);
			season.structural((file) => {
				file.sequence ??= [];
				file.sequence.push({ type: 'chapter', number, status: Status.DRAFT });
			});
			await season.save(`add chapter ${number}`);
			message = m['creator.new_chapter_created']({ number });
			await goto(`/${toonId}/chapter/${number}`);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	function setStatus(index: number, status: string) {
		season?.setShared(['sequence', index, 'status'], status);
	}

	async function saveSeason() {
		if (!season) return;
		busy = true;
		error = null;
		try {
			await season.save();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	async function publishSeason() {
		if (!season) return;
		if (!confirm(m['creator.publish_confirm']({ file: 'season1.json', locales: season.locales.join(', ') }))) return;
		busy = true;
		error = null;
		try {
			if (season.dirty) await season.save();
			await season.publish();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="creator-page">
	<header class="creator-top">
		<nav class="crumbs"><a href="/">{m['creator.title']()}</a> / <strong>{toonId}</strong></nav>
	</header>
	<main class="browser">
		<section>
			{#if !season || season.loading}
				<p>{m['creator.loading']()}</p>
			{:else if season.error}
				<p class="error">{season.error}</p>
			{:else}
				<div class="season-head">
					<div>
						<h2>{m['creator.season']({ number: 1 })}: {season.data[SOURCE_LOCALE]?.title}</h2>
						<p class="hint">
							{#each season.locales as l}{l}: {season.data[l]?.title ?? '–'} {/each}
						</p>
					</div>
					<div class="actions">
						<FileStatus doc={season} />
					</div>
				</div>

				<h3>{m['creator.chapters']()}</h3>
				<table class="chapters">
					<thead>
						<tr><th>#</th>{#each LOCALES as l}<th>{l}</th>{/each}<th>release</th><th>status (season)</th><th>file</th><th></th></tr>
					</thead>
					<tbody>
						{#each sequence as item, index (item.number)}
							{@const doc = chapters[item.number]}
							<tr>
								<td>{item.number}</td>
								{#each LOCALES as l}
									<td>{doc?.data?.[l]?.title ?? '–'}</td>
								{/each}
								<td>{doc?.data?.[SOURCE_LOCALE]?.release ?? ''}</td>
								<td>
									<select value={item.status} onchange={(e) => setStatus(index, e.currentTarget.value)}>
										{#each Object.values(Status) as s}<option value={s}>{s}</option>{/each}
									</select>
								</td>
								<td>{#if doc}<FileStatus {doc} compact />{/if}</td>
								<td><a class="btn" href="/{toonId}/chapter/{item.number}" data-testid="open-chapter-{item.number}">{m['creator.open']()}</a></td>
							</tr>
						{/each}
					</tbody>
				</table>
				{#if message}<p class="hint">{message}</p>{/if}
			{/if}
			{#if error}<p class="error">{error}</p>{/if}
		</section>
		<aside>
			<StyleGuide />
		</aside>
	</main>
	<!-- Bottom action bar: back to the toons · new chapter · season save / publish -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			<a class="bar-link" href="/">← {m['creator.toons']()}</a>
			{#if season?.dirty}<span class="hint">{m['creator.unsaved']()}</span>{/if}
		{/snippet}
		{#snippet secondary()}
			<button onclick={createChapter} disabled={busy || !season || season.loading} data-testid="new-chapter">+ {m['creator.new_chapter']()}</button>
		{/snippet}
		{#snippet end()}
			<button class="secondary" disabled={busy || !season?.dirty} onclick={saveSeason}>{m['creator.save_draft']()}</button>
			<button class="secondary" disabled={busy || !season?.stored.length} onclick={publishSeason}>{m['creator.publish']()}</button>
		{/snippet}
	</ActionBar>
</div>

<style>
	.creator-top {
		display: flex;
		align-items: center;
		min-height: 64px;
		padding: 0 1rem 0 96px;
		border-bottom: 2px solid var(--ink);
	}
	.crumbs a,
	.bar-link {
		color: var(--ink);
	}
	.creator-page {
		--action-bar-height: 64px;
	}
	.browser {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 22rem;
		gap: 2rem;
		padding: 1.5rem 1rem;
		max-width: 1300px;
	}
	@media (max-width: 1000px) {
		.browser {
			grid-template-columns: 1fr;
		}
	}
	.season-head {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		align-items: flex-start;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}
	.chapters {
		width: 100%;
		border-collapse: collapse;
	}
	.chapters th,
	.chapters td {
		text-align: left;
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border-subtle);
	}
	.chapters th {
		border-bottom: 2px solid var(--ink);
		font-size: 11px;
	}
	.chapters tr:hover td {
		background: var(--surface-hover);
	}
	.chapters select {
		width: auto;
	}
	.error {
		color: var(--error);
	}
</style>
