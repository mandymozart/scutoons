<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';

	/**
	 * The saved state of an entity per locale as words (owner 2026-09-30: no version numbers anywhere – the ids are the
	 * store's row ids and stay internal): "Published", "Draft · not published" (newer unpublished changes), "Not
	 * published" (never published), "Missing", "not saved"; the title adds the date of the last change.
	 */
	let { doc, compact = false }: { doc: ContentDoc; compact?: boolean } = $props();

	const dirty = $derived(doc.dirtyLocales);

	function label(l: string) {
		const source = doc.source[l];
		if (source === 'missing') return m['creator.state_missing']();
		if (source === 'new') return m['creator.not_saved']();
		if (doc.published[l] == null) return m['creator.state_unpublished']();
		return doc.base[l] === doc.published[l] ? m['creator.state_published']() : m['creator.state_draft']();
	}
	const date = (iso: string | null | undefined) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }).format(d) : '';
	};
	/** the language-neutral entities (credits, characters, cast …) carry no language */
	const lang = (l: string) => (l === '_' ? '' : l);
	function title(l: string) {
		const when = doc.source[l] === 'store' ? date(doc.changedAt[l]) : '';
		return `${lang(l) || '·'}: ${label(l)}${when ? ` · ${m['creator.state_changed']({ date: when })}` : ''}`;
	}
</script>

<span class="file-status" class:compact>
	{#each doc.locales as l}
		<span
			class="badge"
			class:dirty={dirty.includes(l)}
			class:live={doc.source[l] === 'store' && doc.base[l] === doc.published[l]}
			class:pending={doc.source[l] === 'store' && doc.base[l] !== doc.published[l]}
			class:missing={doc.source[l] === 'missing'}
			title={title(l)}
		>
			{#if compact}{lang(l) || label(l)}{:else}{lang(l) ? `${l} · ` : ''}{label(l)}{/if}{dirty.includes(l) ? ' •' : ''}
		</span>
	{/each}
</span>

<style>
	.file-status {
		display: inline-flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}
	.badge {
		font-size: 10px;
		padding: 0.1em 0.4em;
		border: 1px solid var(--border-muted);
		white-space: nowrap;
	}
	.badge.live {
		border-color: var(--ink);
	}
	.badge.pending {
		border-style: dashed;
		border-color: var(--ink);
	}
	.badge.dirty {
		background: var(--ink);
		color: var(--paper);
	}
	.badge.missing {
		color: var(--error);
		border-color: var(--error);
	}
</style>
