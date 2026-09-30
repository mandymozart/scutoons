<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { doc, compact = false }: { doc: ContentDoc; compact?: boolean } = $props();

	const dirty = $derived(doc.dirtyLocales);

	function label(l: string) {
		const source = doc.source[l];
		if (source === 'missing') return m['creator.missing']();
		if (source === 'static') return m['creator.static_file']();
		if (source === 'new') return m['creator.not_saved']();
		const base = doc.base[l];
		return base === doc.published[l] ? m['creator.version_published']({ version: base ?? '' }) : m['creator.version_draft']({ version: base ?? '' });
	}
	/** the language-neutral files (credits, characters, cast …) carry no language */
	const lang = (l: string) => (l === '_' ? '' : l);
	function title(l: string) {
		const source = doc.source[l];
		if (source !== 'store') return `${lang(l) || '·'}: ${label(l)}`;
		return `${lang(l) || '·'}: ${m['creator.in_store']()} v${doc.base[l]}${doc.published[l] ? ` · ${m['creator.published']()} v${doc.published[l]}` : ''}${doc.published[l] !== doc.base[l] ? ` · ${m['creator.unpublished_changes']()}` : ''}`;
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
