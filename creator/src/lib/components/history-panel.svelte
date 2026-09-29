<script lang="ts">
	import { listVersions, publishVersion, restoreVersion, unpublishFile, type ContentVersion } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** Version history of one file per locale: restore an old version as the new draft, publish / unpublish. */
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
					out[l] = doc.source[l] === 'store' ? (await listVersions(doc.toon, l, doc.path)).versions : [];
				})
			);
			versions = out;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		void [doc.path, doc.base[doc.primary], doc.published[doc.primary]];
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

	function restore(locale: string, v: ContentVersion) {
		if (doc.dirtyLocales.includes(locale) && !confirm(m['creator.leave_confirm']())) return;
		if (!confirm(m['creator.restore_confirm']({ id: v.id, locale }))) return;
		act(() => restoreVersion(v.id));
	}

	function publish(locale: string, v: ContentVersion) {
		if (!confirm(m['creator.publish_confirm']({ file: `${doc.path} #${v.id}`, locales: locale }))) return;
		act(() => publishVersion(doc.toon, locale, doc.path, v.id));
	}

	function unpublish(locale: string) {
		if (!confirm(`Unpublish ${doc.path} (${locale})? Readers get the static file again (or nothing).`)) return;
		act(() => unpublishFile(doc.toon, locale, doc.path));
	}

	const when = (iso: string) => iso?.replace('T', ' ').slice(0, 16);
</script>

<div class="history" data-testid="history-panel">
	<header>
		<h2>{m['creator.history']()} · {doc.path}</h2>
		{#if onclose}<button class="ghost" onclick={onclose} aria-label="Close">✕</button>{/if}
	</header>
	{#if error}<p class="error">{error}</p>{/if}
	{#each doc.locales as l (l)}
		<h3>{l}</h3>
		{#if !versions[l]?.length}
			<p class="hint">{doc.source[l] === 'store' ? '…' : `${m['creator.static_file']()} – not in the store yet`}</p>
		{:else}
			<ol>
				{#each versions[l] as v (v.id)}
					<li class:live={v.status === 'published'} class:current={v.id === doc.base[l]}>
						<span class="id">#{v.id}</span>
						<span class="what">
							{when(v.created_at)} · {v.author_email ?? 'import'}
							{#if v.message}<br /><i>{v.message}</i>{/if}
						</span>
						<span class="acts">
							{#if v.status === 'published'}
								<b>live</b>
								<button class="ghost" disabled={busy} onclick={() => unpublish(l)}>unpublish</button>
							{:else}
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
	.history {
		font-size: 11px;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 2px solid var(--ink);
	}
	h2 {
		font-size: 12px;
		margin: 0.25rem 0;
	}
	h3 {
		font-size: 11px;
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
		grid-template-columns: 3rem 1fr auto;
		gap: 0.3rem;
		padding: 0.25rem 0;
		border-bottom: 1px solid var(--border-subtle);
		align-items: start;
	}
	li.current .id {
		text-decoration: underline;
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
		font-size: 10px;
	}
	.error {
		color: var(--error);
	}
</style>
