<script lang="ts">
	import type { Snippet } from 'svelte';
	import { CloseIcon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel, fileLabels } from '$lib/labels.ts';
	import { NEUTRAL_LOCALE, visibleDifferences } from '@scutoons/content';
	import { differenceLine } from '$lib/editor/conversion.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { DocSession } from '$lib/editor/doc-session.svelte.ts';

	/**
	 * The bars under an editor page's file status (COMPONENTS.md step 11; meta pages and the chapter editor had copies):
	 * unsaved changes, an autosave to recover, a document converted from its per-language versions on load (agents/NEUTRAL.md: saving stores the neutral one; what readers will see differently, folded),
	 * events that got their hidden id on load (saving keeps them), a save conflict, the last notice.
	 */
	let {
		session,
		docs,
		dirty,
		dirtyDetail,
		extra
	}: {
		session: DocSession;
		docs: ContentDoc[];
		/** show the "unsaved" bar */
		dirty: boolean;
		/** after "unsaved" (e.g. the slice's languages with changes) */
		dirtyDetail?: string;
		/** more bars at the end (e.g. the chapter's cast notice) */
		extra?: Snippet;
	} = $props();

	const recoverables = $derived(docs.filter((d) => d.recoverable));
	/** documents converted in memory that are not stored yet – one bar for all of them */
	const conversions = $derived(docs.filter((d) => d.unconverted));
	/** events that got their hidden id on load (agents/EXPRESSIONS.md §5): the next save stores them */
	const withNewIds = $derived(docs.filter((d) => d.idsAssigned > 0));
	const differences = $derived(conversions.flatMap((d) => visibleDifferences(d.converted!.report).map((r) => ({ file: fileLabel(d.entity), line: differenceLine(r) }))));
	/** "Chapter 2", "Script (en)": a language only for a document stored per language (not the neutral `_`) */
	const conflictLabel = (locale: string | undefined, entity: string) => (locale && locale !== NEUTRAL_LOCALE ? `${fileLabel(entity)} (${locale})` : fileLabel(entity));
</script>

{#if dirty}<div class="sc-bar is-dirty">{m['creator.unsaved']()}{#if dirtyDetail} · {dirtyDetail}{/if}</div>{/if}
{#each recoverables as d (d.entity)}
	<div class="sc-bar is-warning">
		{fileLabel(d.entity)}: {m['creator.recover_title']({ time: new Date(d.recoverable!.at).toLocaleString() })}
		{#if d.recoverableIsStale}<strong class="sc-emphasis">{m['creator.recover_stale']()}</strong>{/if}
		<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
		<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
	</div>
{/each}
{#if conversions.length}
	<div class="sc-bar is-note conversion" data-testid="converted">
		<span>{m['creator.converted_notice']({ files: fileLabels(conversions.map((d) => d.entity)) })}</span>
		{#if differences.length}
			<details>
				<summary>{m['creator.converted_differences']({ count: differences.length })}</summary>
				<ul>
					{#each differences as d, i (i)}<li>{d.file} · {d.line}</li>{/each}
				</ul>
			</details>
		{:else}
			<span>{m['creator.converted_no_differences']()}</span>
		{/if}
	</div>
{/if}
{#if withNewIds.length}
	<div class="sc-bar is-warning" data-testid="event-ids-assigned">
		{m['creator.event_ids_assigned']({ count: withNewIds.reduce((n, d) => n + d.idsAssigned, 0), files: fileLabels(withNewIds.map((d) => d.entity)) })}
	</div>
{/if}
{#if session.conflict}
	<div class="sc-bar is-error">
		{m['creator.conflict']({ file: conflictLabel(session.conflict.error.locale, session.conflict.doc.entity) })}
		<button class="secondary" onclick={() => session.resolveConflict('reload')}>{m['creator.reload']()}</button>
		<button class="secondary" onclick={() => session.resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
	</div>
{/if}
{#if session.notice}<div class="sc-bar is-note">{session.notice} <button class="ghost" onclick={() => (session.notice = null)} aria-label={m['creator.close']()} title={m['creator.close']()}><CloseIcon /></button></div>{/if}
{@render extra?.()}

<style>
	.conversion {
		flex-wrap: wrap;
	}
	.conversion details {
		flex-basis: 100%;
	}
	.conversion ul {
		max-height: 12rem;
		overflow: auto;
		margin: var(--space-05) 0 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
</style>
