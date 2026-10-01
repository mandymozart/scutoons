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
		gap: var(--space-05);
		flex-wrap: wrap;
	}
	/* status chips (DESIGN.md "Brutalist tools"), filled from tokens.css --badge-*: published · saved draft (grey) ·
	   unsaved (warning) · missing (error); anything else neutral (black) */
	.badge {
		padding: 0.1em 0.4em;
		border: var(--border-width-regular) solid currentColor;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		border: 0;
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
		white-space: nowrap;
	}
	.badge.live {
		background: var(--badge-published);
		color: var(--badge-published-text);
	}
	.badge.pending {
		background: var(--badge-draft);
		color: var(--badge-draft-text);
	}
	.badge.dirty {
		background: var(--badge-warning);
		color: var(--badge-warning-text);
	}
	.badge.missing {
		background: var(--badge-error);
		color: var(--badge-error-text);
	}
</style>
