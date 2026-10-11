<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel, fileLabels } from '$lib/labels.ts';
	import { NEUTRAL_LOCALE, visibleDifferences } from '@scutoons/content';
	import { differenceLine } from '$lib/editor/conversion.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { DocSession } from '$lib/editor/doc-session.svelte.ts';

	/**
	 * An editor page's blocking problems – the only bars left in the page flow (UI-CONCEPTS E.1; COMPONENTS.md step 11:
	 * meta pages and the chapter editor share it): an autosave to recover, a document converted from its per-language
	 * versions on load (agents/NEUTRAL.md: saving stores the neutral one; what readers will see differently, folded), a
	 * save conflict (Reload / Overwrite). Unsaved changes = the Save button's dot, the not-live languages = the ActionBar's
	 * file-status chip, Saved / Published / errors = toasts (DocSession). Events' hidden ids: nothing shown.
	 */
	let { session, docs }: { session: DocSession; docs: ContentDoc[] } = $props();

	const recoverables = $derived(docs.filter((d) => d.recoverable));
	/** documents converted in memory that are not stored yet – one bar for all of them */
	const conversions = $derived(docs.filter((d) => d.unconverted));
	// events that got their hidden id on load (agents/EXPRESSIONS.md §5) are stored by the next save – silently
	// (owner 2026-10-11: "ich muss nichts mit IDs zu tun haben")
	const differences = $derived(conversions.flatMap((d) => visibleDifferences(d.converted!.report).map((r) => ({ file: fileLabel(d.entity), line: differenceLine(r) }))));
	/** "Chapter 2", "Script (en)": a language only for a document stored per language (not the neutral `_`) */
	/** "10/11/26, 7:24 PM" – short, so the notice stays on one line (owner 2026-10-11) */
	const shortTime = (at: string | number) => new Date(at).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' });
	const conflictLabel = (locale: string | undefined, entity: string) => (locale && locale !== NEUTRAL_LOCALE ? `${fileLabel(entity)} (${locale})` : fileLabel(entity));
</script>

{#each recoverables as d (d.entity)}
	{@const text = `${fileLabel(d.entity)}: ${m['creator.recover_title']({ time: shortTime(d.recoverable!.at) })}${d.recoverableIsStale ? ` (${m['creator.recover_stale']()})` : ''}`}
	<div class="sc-bar is-warning line" data-testid="recover">
		<span class="text" title={text}>{text}</span>
		<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
		<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
	</div>
{/each}
{#if conversions.length}
	<div class="sc-bar is-note line conversion" data-testid="converted">
		<span class="text">{m['creator.converted_notice']({ files: fileLabels(conversions.map((d) => d.entity)) })}</span>
		{#if differences.length}
			<details>
				<summary>{m['creator.converted_differences']({ count: differences.length })}</summary>
				<ul>
					{#each differences as d, i (i)}<li>{d.file} · {d.line}</li>{/each}
				</ul>
			</details>
		{:else}
			<span class="text">{m['creator.converted_no_differences']()}</span>
		{/if}
	</div>
{/if}
{#if session.conflict}
	{@const text = m['creator.conflict']({ file: conflictLabel(session.conflict.error.locale, session.conflict.doc.entity) })}
	<div class="sc-bar is-error line" data-testid="conflict">
		<span class="text" title={text}>{text}</span>
		<button class="secondary" onclick={() => session.resolveConflict('reload')}>{m['creator.reload']()}</button>
		<button class="secondary" onclick={() => session.resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
	</div>
{/if}

<style>
	/* one line at most (owner 2026-10-11: "these messages use too much real estate"): the text shortens with an
	   ellipsis (full text in the tooltip), the actions stay; only an opened list of differences takes more room */
	.line {
		flex-wrap: nowrap;
		padding-block: var(--space-025);
	}
	.line .text {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.line > button,
	.line > details {
		flex-shrink: 0;
	}
	.conversion:has(details[open]) {
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
