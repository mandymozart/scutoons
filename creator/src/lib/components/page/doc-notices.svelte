<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { DocSession } from '$lib/editor/doc-session.svelte.ts';

	/**
	 * An editor page's blocking problems – the only bars left in the page flow (UI-CONCEPTS E.1; COMPONENTS.md step 11:
	 * meta pages and the chapter editor share it): a local copy to recover. Edits save on their own, what is not live is
	 * the Publish pill's dot, Saved / Published / errors are Log lines, a newer version – a refused save too – is the
	 * shell's Reload notice (DocSession).
	 */
	let { session, docs }: { session: DocSession; docs: ContentDoc[] } = $props();

	const recoverables = $derived(docs.filter((d) => d.recoverable));
	/** "10/11/26, 7:24 PM" – short, so the notice stays on one line */
	const shortTime = (at: string | number) => new Date(at).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' });
</script>

{#each recoverables as d (d.entity)}
	{@const text = `${fileLabel(d.entity)}: ${m['creator.recover_title']({ time: shortTime(d.recoverable!.at) })}${d.recoverableIsStale ? ` (${m['creator.recover_stale']()})` : ''}`}
	<div class="sc-bar is-warning line" data-testid="recover">
		<span class="text" title={text}>{text}</span>
		<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
		<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
	</div>
{/each}

<style>
	/* one line at most (owner 2026-10-11: "these messages use too much real estate"): the text shortens with an
	   ellipsis (full text in the tooltip), the actions stay */
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
	.line > button {
		flex-shrink: 0;
	}
</style>
