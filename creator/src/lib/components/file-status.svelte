<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';

	/**
	 * The saved state of an entity per locale as words (owner 2026-09-30: no version numbers anywhere – the ids are the
	 * store's row ids and stay internal): "Published", "Draft · not published" (newer unpublished changes), "Not
	 * published" (never published), "Missing", "not saved"; the title adds the date of the last change.
	 * Owner 2026-10-02 ("we got color coding now … more white space"): the chip shows only the language, its colour is
	 * the state; the words are a tooltip on hover and on keyboard focus (and the accessible name). `full` keeps the words.
	 */
	let { doc, full = false, label: name }: { doc: ContentDoc; full?: boolean; label?: string } = $props();

	/** with a `label` (owner 2026-10-02): the label and the chips are one hover / focus target, the tooltip lists every
	 * language (easier to hit, one accessible name) */
	const groupTip = $derived(name ? `${name} – ${doc.locales.map((l) => title(l)).join(' · ')}` : '');

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
		return `${lang(l) ? `${lang(l)}: ` : ''}${label(l)}${when ? ` · ${m['creator.state_changed']({ date: when })}` : ''}`;
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
	class="file-status"
	class:labelled={!!name}
	tabindex={name && !full ? 0 : undefined}
	role={name && !full ? 'img' : undefined}
	aria-label={name && !full ? groupTip : undefined}
	data-tip={name && !full ? groupTip : undefined}
>
	{#if name}<span class="name">{name}</span>{/if}
	{#each doc.locales as l}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
		<span
			class="badge"
			class:dirty={dirty.includes(l)}
			class:live={doc.source[l] === 'store' && doc.base[l] === doc.published[l]}
			class:pending={doc.source[l] === 'store' && doc.base[l] !== doc.published[l]}
			class:missing={doc.source[l] === 'missing'}
			class:neutral={!lang(l)}
			tabindex={full || name ? undefined : 0}
			role={full || name ? undefined : 'img'}
			aria-label={full || name ? undefined : title(l)}
			data-tip={full || name ? undefined : title(l)}
			aria-hidden={name && !full ? 'true' : undefined}
			title={full ? title(l) : undefined}
		>
			{#if full}{lang(l) ? `${l} · ` : ''}{label(l)}{:else}{lang(l)}{/if}{dirty.includes(l) ? ' •' : ''}
		</span>
	{/each}
</span>

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
	/* status chips (DESIGN.md "Brutalist tools"), filled from tokens.css --badge-*: published · saved draft (grey) ·
	   unsaved (warning) · missing (error); anything else neutral (black) */
	.badge {
		display: inline-block;
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
	/* a language-neutral entity: just the colour */
	.badge.neutral {
		width: 0.75rem;
		height: 0.75rem;
		padding: 0;
		align-self: center;
	}
	/* the words as a tooltip: hover and keyboard focus */
	[data-tip] {
		position: relative;
		cursor: default;
	}
	[data-tip]:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: 1px;
	}
	[data-tip]:hover::after,
	[data-tip]:focus-visible::after {
		content: attr(data-tip);
		position: absolute;
		z-index: 40;
		top: calc(100% + 0.25rem);
		left: 0;
		padding: 0.25rem 0.5rem;
		background: var(--ink);
		color: var(--paper);
		text-transform: none;
		letter-spacing: normal;
		white-space: nowrap;
		pointer-events: none;
	}
</style>
