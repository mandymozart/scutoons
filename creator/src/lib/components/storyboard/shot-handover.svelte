<script lang="ts">
	// A shot hands over by a link (storyboard-handover.ts): its slice in the chapter (cut there on first use), the deck (its
	// scene, else the shot itself – the deck makes the scene, #873) and its XR experience. Where an address opens in the
	// deck is the core's (deck-catalog `library::open`): a made scene's own answer, else where a scene opens.
	import { Button } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { library } from '$lib/editor/library.svelte.ts';
	import { onDeckLink } from '$lib/shell/deck.ts';
	import { deckTarget, shotDeckLink, sliceHref, xrHref } from '$lib/editor/storyboard-handover.ts';
	import { AREAS, hrefOf } from '$lib/shell/address.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const entry = $derived(sb.sel);
	const target = $derived(entry ? deckTarget(entry.shot.made, sb.toon, sb.chapter, entry.id) : null);
	const xr = $derived(entry ? xrHref(entry.shot.made) : null);

	let deckHref = $state<string | null>(null);
	let why = $state<string | null>(null);
	$effect(() => {
		const asked = target;
		if (!asked) return;
		let current = true;
		library
			.opener('deck')
			.then((rule) => {
				if (!current) return;
				const answer = asked.made ? rule.open(asked.address) : null;
				const workspace = asked.made ? (answer?.ok ? answer.workspace : null) : (rule.kinds().find((k) => k.kind === 'scene')?.workspace ?? null);
				deckHref = shotDeckLink(workspace, asked.address);
				why = answer && !answer.ok ? answer.why : null;
			})
			.catch(() => current && ((deckHref = null), (why = null)));
		return () => (current = false);
	});
</script>

{#if entry}
	<div class="handover" data-testid="shot-handover">
		<Button variant="quiet" icon="columns" href={sliceHref(sb.toon, sb.chapter, entry.id)} title={m['storyboard.handover_slice_hint']()} data-testid="handover-slice">{m['storyboard.handover_slice']()}</Button>
		<Button
			variant="quiet"
			icon="external-link"
			href={deckHref ?? undefined}
			disabled={!deckHref}
			title={why ?? (target?.made ? m['storyboard.handover_deck_hint']() : m['storyboard.handover_deck_new_hint']())}
			onclick={onDeckLink}
			data-testid="handover-deck">{m['storyboard.handover_deck']()}</Button
		>
		<Button variant="quiet" icon="aperture" href={xr ?? hrefOf(AREAS.experiences) ?? undefined} title={xr ? m['storyboard.handover_xr_hint']() : m['storyboard.handover_xr_none_hint']()} data-testid="handover-xr">{m['storyboard.handover_xr']()}</Button>
	</div>
{/if}

<style>
	.handover {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
</style>
