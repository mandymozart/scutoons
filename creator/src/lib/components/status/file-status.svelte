<script lang="ts" module>
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** a language is live: published as it is, no unsaved edits */
	export const isLive = (doc: ContentDoc, l: string) =>
		doc.source[l] === 'store' && doc.base[l] === doc.published[l] && !doc.dirtyLocales.includes(l);
	/** the file has something to show (not every language live) – the status row only then (owner 2026-10-03) */
	export const hasStatus = (doc: ContentDoc) => !doc.loading && doc.locales.some((l) => !isLive(doc, l));
	/** what keeps a language from being live, the worst first (the ActionBar's chip shows the worst – live-chip.svelte) */
	export const STATES = ['missing', 'unsaved', 'draft'] as const;
	export type NotLive = (typeof STATES)[number];
	export const notLive = (doc: ContentDoc, l: string): NotLive | null =>
		isLive(doc, l) ? null
		: doc.source[l] === 'missing' ? 'missing'
		: doc.dirtyLocales.includes(l) || doc.source[l] === 'new' ? 'unsaved'
		: 'draft';
	/** a language's state in words: "Published", "Draft · not published", "Not published", "Missing", "not saved" */
	export function stateWords(doc: ContentDoc, l: string) {
		const source = doc.source[l];
		if (source === 'missing') return m['creator.state_missing']();
		if (source === 'new') return m['creator.not_saved']();
		if (doc.published[l] == null) return m['creator.state_unpublished']();
		return doc.base[l] === doc.published[l] ? m['creator.state_published']() : m['creator.state_draft']();
	}
</script>

<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime.js';

	/**
	 * The saved state of an entity per locale as words (owner 2026-09-30: no version numbers anywhere – the ids are the
	 * store's row ids and stay internal): "Published", "Draft · not published" (newer unpublished changes), "Not
	 * published" (never published), "Missing", "not saved"; the title adds the date of the last change.
	 * Owner 2026-10-02 ("we got color coding now … more white space"): the chip shows only the language, its colour is
	 * the state; the words are a tooltip (the universal tool tooltip) on hover and on keyboard focus (and the accessible name). `full` keeps the words.
	 */
	let { doc, full = false, label: name }: { doc: ContentDoc; full?: boolean; label?: string } = $props();

	/** with a `label` (owner 2026-10-02): the label and the chips are one hover / focus target, the tooltip lists every
	 * language (easier to hit, one accessible name) */
	const groupTip = $derived.by(() => {
		if (!name) return '';
		// owner 2026-10-02: one date – the last change of any language, not one per language
		const latest = doc.locales
			.filter((l) => doc.source[l] === 'store')
			.map((l) => doc.changedAt[l])
			.filter((iso): iso is string => !!iso)
			.sort((a, b) => new Date(a).getTime() - new Date(b).getTime())
			.at(-1);
		const states = doc.locales.map((l) => `${lang(l) ? `${lang(l)}: ` : ''}${label(l)}${unsaved(l)}`).join(' · ');
		// owner 2026-10-02: the label is right there – the tooltip is only the state and when it changed
		return `${states}${latest ? ` · ${m['creator.state_changed']({ date: date(latest) })}` : ''}`;
	});

	const dirty = $derived(doc.dirtyLocales);
	/**
	 * owner 2026-10-03: the chips are only for what is NOT published – a language that is live and has no unsaved edits
	 * shows nothing; a fully published file shows nothing at all (label included). `full` lists every language.
	 */
	const shown = $derived(full ? doc.locales : doc.locales.filter((l) => !isLive(doc, l)));

	const label = (l: string) => stateWords(doc, l);
	const date = (iso: string | null | undefined) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }).format(d) : '';
	};
	/** the language-neutral entities (credits, characters, cast …) carry no language */
	const lang = (l: string) => (l === '_' ? '' : l);
	/** the • of a language with unsaved edits, in words for the tooltip and screen readers */
	function unsaved(l: string) {
		return dirty.includes(l) && doc.source[l] !== 'new' ? ` (${m['creator.unsaved']()})` : '';
	}
	/** the chip's colour = its state (tools.css .sc-chip): missing · unsaved · saved draft · published; a language-neutral
	 * entity is just the colour (a square) */
	function tone(l: string) {
		const source = doc.source[l];
		const state =
			source === 'missing' ? 'is-error'
			: dirty.includes(l) ? 'is-warning'
			: source === 'store' && doc.base[l] !== doc.published[l] ? 'is-draft'
			: source === 'store' ? 'is-published'
			: '';
		return lang(l) ? state : `${state} is-dot`;
	}
	function title(l: string) {
		const when = doc.source[l] === 'store' ? date(doc.changedAt[l]) : '';
		return `${lang(l) ? `${lang(l)}: ` : ''}${label(l)}${unsaved(l)}${when ? ` · ${m['creator.state_changed']({ date: when })}` : ''}`;
	}
</script>

{#if shown.length}
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
	class="file-status"
	class:labelled={!!name}
	tabindex={name && !full ? 0 : undefined}
	role={name && !full ? 'img' : undefined}
	aria-label={name && !full ? `${name} – ${groupTip}` : undefined}
	title={name && !full ? groupTip : undefined}
>
	{#if name}<span class="name">{name}</span>{/if}
	{#each shown as l}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<span
			class="sc-chip {tone(l)}"
			tabindex={full || name ? undefined : 0}
			role={full || name ? undefined : 'img'}
			aria-label={full || name ? undefined : title(l)}
			aria-hidden={name && !full ? 'true' : undefined}
			title={name && !full ? undefined : title(l)}
		>
			{#if full}{lang(l) ? `${l} · ` : ''}{label(l)}{:else}{lang(l)}{/if}{dirty.includes(l) ? ' •' : ''}
		</span>
	{/each}
</span>
{/if}

<style>
	.file-status {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		flex-wrap: wrap;
	}
	.file-status .name {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	/* the chips are tools.css .sc-chip; the words are the universal tool tooltip (@scutoons/ui installTooltips, on hover
	   and keyboard focus) */
	.file-status[tabindex]:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: 1px;
	}
</style>
