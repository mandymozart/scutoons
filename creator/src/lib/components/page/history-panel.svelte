<script lang="ts">
	import { reviews } from '$lib/editor/reviews.svelte.ts';
	import { confirmDialog } from '@scutoons/ui';
	import { CloseIcon } from '@scutoons/ui';
	import { listVersions, publishVersion, restoreVersion, unpublish as unpublishEntity, type ContentVersion } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';
	import { NEUTRAL_LOCALE } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';

	/** Version history of one entity per locale: restore an old version as the new draft, publish / unpublish. */
	let { doc, onclose }: { doc: ContentDoc; onclose?: () => void } = $props();

	let versions = $state<Record<string, ContentVersion[]>>({});
	let error = $state<string | null>(null);
	let busy = $state(false);

	async function load() {
		error = null;
		try {
			const out: Record<string, ContentVersion[]> = {};
			await Promise.all(
				doc.locales.map(async (l) => {
					out[l] = doc.source[l] === 'store' ? (await listVersions(doc.key(l))).versions : [];
				})
			);
			versions = out;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		void [doc.entity, doc.base[doc.primary], doc.published[doc.primary]];
		load();
	});

	async function act(work: () => Promise<unknown>) {
		busy = true;
		error = null;
		try {
			await work();
			await doc.load();
			await load();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	// review before publish: accounts that need a review submit through the page's button (no publish / unpublish here);
	// versions waiting for a review are marked
	const reviewed = $derived(reviews.needsReview(doc.toon));
	const inReview = $derived(new Set([...(reviews.overview?.to_review ?? []), ...(reviews.overview?.mine ?? [])].filter((r) => r.state === 'pending').map((r) => r.version)));

	/** a language-neutral document's version holds every language (chapters, scenes, seasons: the toon's) */
	const lang = (locale: string) => (locale === NEUTRAL_LOCALE ? (doc.texts?.languages.join(', ') ?? '–') : locale);

	async function restore(locale: string, v: ContentVersion) {
		// one question: unsaved changes in that language are lost, and the version becomes a new draft
		const message = `${doc.dirtyLocales.includes(locale) ? `${m['creator.leave_confirm']()} ` : ''}${m['creator.restore_confirm_when']({ when: when(v.created_at), locale: lang(locale) })}`;
		if (!(await confirmDialog({ title: m['creator.restore_title'](), message, confirmLabel: m['creator.restore']() }))) return;
		act(() => restoreVersion(doc.key(locale), v.id));
	}

	async function publish(locale: string, v: ContentVersion) {
		if (!(await confirmDialog({ title: m['creator.publish_title'](), message: m['creator.publish_confirm']({ file: `${fileLabel(doc.entity)} (${when(v.created_at)})`, locales: lang(locale) }), confirmLabel: m['creator.publish']() }))) return;
		act(() => publishVersion(doc.key(locale), v.id));
	}

	async function unpublish(locale: string) {
		if (!(await confirmDialog({ title: m['creator.unpublish_title'](), message: m['creator.unpublish_confirm']({ file: fileLabel(doc.entity), locale: lang(locale) }), confirmLabel: m['creator.unpublish_label']() }))) return;
		act(() => unpublishEntity(doc.key(locale)));
	}

	/** versions are told apart by date, author and note – never by their (store-wide) id */
	const when = (iso: string) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : (iso ?? '');
	};
</script>

<div class="history" data-testid="history-panel">
	<header>
		<h2>{m['creator.history']()} · {fileLabel(doc.entity)}</h2>
		{#if onclose}<button class="ghost" onclick={onclose} aria-label="Close"><CloseIcon /></button>{/if}
	</header>
	{#if error}<p class="error">{error}</p>{/if}
	{#each doc.locales as l (l)}
		{#if l !== NEUTRAL_LOCALE}<h3>{l}</h3>{/if}
		{#if !versions[l]?.length}
			<p class="hint">{doc.source[l] === 'store' ? '…' : m['creator.static_file']()}</p>
		{:else}
			<ol>
				{#each versions[l] as v (v.id)}
					<li class:live={v.status === 'published'} class:current={v.id === doc.base[l]}>
						<span class="what">
							<span>{when(v.created_at)}</span>{#if v.id === doc.base[l]} <span class="tag">{m['creator.history_current']()}</span>{/if}{#if inReview.has(v.id)} <span class="tag">{m['creator.review_state_pending']()}</span>{/if}
							<br />{v.author_email ? m['creator.history_by']({ author: v.author_email }) : m['creator.history_import']()}
							{#if v.message}<br /><i>{v.message}</i>{/if}
						</span>
						<span class="acts">
							{#if v.status === 'published'}
								<span>{m['creator.state_published']()}</span>
								{#if !reviewed}<button class="ghost" disabled={busy} onclick={() => unpublish(l)}>unpublish</button>{/if}
							{:else if !reviewed}
								<button class="ghost" disabled={busy} onclick={() => publish(l, v)}>{m['creator.publish']()}</button>
							{/if}
							{#if v.id !== doc.base[l]}
								<button class="ghost" disabled={busy} onclick={() => restore(l, v)} data-testid="restore-{l}-{v.id}">{m['creator.restore']()}</button>
							{/if}
						</span>
					</li>
				{/each}
			</ol>
		{/if}
	{/each}
</div>

<style>
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 2px solid var(--ink);
	}
	h2 {
		margin: 0.25rem 0;
	}
	h3 {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		margin: 0.6rem 0 0.2rem;
		text-transform: uppercase;
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 0.3rem;
		padding: 0.25rem 0;
		border-bottom: 1px solid var(--border-subtle);
		align-items: start;
	}
	.tag {
		margin-left: 0.4rem;
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		border: 1px solid var(--ink);
	}
	li.live {
		border-left: 3px solid var(--ink);
		padding-left: 0.2rem;
	}
	.acts {
		display: flex;
		gap: 0.2rem;
		align-items: center;
	}
	.acts button {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.error {
		color: var(--error);
	}
</style>
