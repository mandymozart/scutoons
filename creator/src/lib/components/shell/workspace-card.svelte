<script lang="ts">
	import { CloseIcon, Icon, ToonAvatar } from '@scutoons/ui';
	import { Chip, IconButton, Thumb } from '@scutoons/ui/editor';
	import { workspaceThumbKey } from '@scutoons/api';
	import { toonAsset } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf, workspaceOf } from '$lib/shell/address.ts';
	import type { WorkspaceCard } from '$lib/shell/dashboard.ts';
	import { onDeckLink, type DeckCard } from '$lib/shell/deck.ts';
	import { snapshots } from '$lib/shell/snapshots.svelte.ts';
	import { thumbs } from '$lib/shell/thumbs.svelte.ts';
	import { deckWorkspaceName, workspaceName } from '$lib/shell/workspace-names.ts';

	/**
	 * A workspace in the switcher (lap2 B7): its last capture fills the card, its name and what is open in it over the
	 * picture; without a capture yet the item's toon and its kind's icon. ✕ closes that item. Views are in the ☰ menu only.
	 * A deck card (A3) carries the quiet chip "Deck" and is the link that opens the deck there (A3b: the browser asks once).
	 */
	let {
		card,
		ring = false,
		onopen,
		onclose,
		onfocus
	}: { card: WorkspaceCard | DeckCard; ring?: boolean; onopen: (card: WorkspaceCard) => void; onclose: (address: string) => void; onfocus: () => void } = $props();

	const nav = getCreatorNav();
	const deck = $derived('workspace' in card ? card : null);
	const own = $derived('workspace' in card ? null : card);
	const name = $derived(deck ? deckWorkspaceName(deck.workspace) : own ? (own.name ?? workspaceName(own.kind)) : '');
	const item = $derived(own?.shown ?? null);
	const address = $derived(deck ? deck.item : (item?.address ?? null));
	const toon = $derived(item ? (item.address.split(':')[1] ?? null) : null);
	const avatar = $derived.by(() => {
		const ws = item ? workspaceOf(item.address) : null;
		const t = toon && (ws === 'toon' || ws === 'chapter' || ws === 'storyboard') ? nav.toons.find((x) => x.id === toon) : undefined;
		return t?.avatar ? asset(toonAsset(t.id, t.avatar)) : null;
	});
	const still = $derived(snapshots.local[card.id] ?? thumbs.get(workspaceThumbKey(card.id))?.url ?? null);
	const more = $derived(own && own.open.length > 1 ? ` +${own.open.length - 1}` : '');
	const shown = $derived(deck ? deck.item : item ? `${item.label}${more}` : null);
	const href = $derived(deck ? deck.href : (own?.href ?? null));
	function click(event: MouseEvent) {
		if (deck) onDeckLink(event);
		else if (own) onopen(own);
	}
</script>

<div class="ws-card" class:front={own?.front} class:empty={!address} class:captured={!!still} data-testid="dash-card" data-kind={own?.kind ?? 'deck'} data-workspace={card.id}>
	{#snippet inside()}
		<Thumb src={still} fill empty={m['shell.no_still']()}>
			<span class="placeholder">
				{#if avatar}<ToonAvatar src={avatar} name={toon ?? ''} size="2.6rem" />{/if}
				<Icon name={address ? iconOf(address) : 'layout'} size="2rem" strokeWidth={1.25} />
			</span>
		</Thumb>
		{#if own?.front}<span class="tag">{m['shell.dash_front']()}</span>{/if}
		<span class="words">
			<span class="word">{name}</span>
			<span class="quiet">{shown ?? m['shell.dash_nothing_open']()}</span>
		</span>
	{/snippet}
	{#if href}
		<a class="open" {href} title={deck ? m['shell.deck_card_title']() : undefined} data-ring={ring || undefined} onclick={click} {onfocus} data-testid="dash-card-open">{@render inside()}</a>
	{:else}
		<button type="button" class="open" data-ring={ring || undefined} onclick={click} {onfocus} data-testid="dash-card-open">{@render inside()}</button>
	{/if}
	{#if deck}
		<span class="deck" aria-hidden="true"><Chip label={m['shell.deck_chip']()} tabindex={-1} testid="dash-card-deck" /></span>
	{:else if item}
		<span class="close">
			<IconButton label={m['shell.close_tab']({ name: item.label })} onclick={() => onclose(item.address)} tabindex={-1} data-testid="dash-card-close"><CloseIcon size="0.8rem" stroke={1} /></IconButton>
		</span>
	{/if}
</div>

<style>
	/* a card on the stage (kit/tokens.toml `[component.workspace_card.*]`; deck-kit home.rs): the capture is its ground */
	.ws-card {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		aspect-ratio: 6 / 5;
		border: var(--editor-edge) solid var(--kit-workspace-card-rest-border);
		border-radius: var(--kit-radius-box);
		background: var(--tool-panel);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
	}
	.ws-card :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.ws-card.empty {
		border-style: dashed;
	}
	.open {
		position: relative;
		display: flex;
		flex-direction: column;
		flex: 1;
		justify-content: flex-end;
		min-width: 0;
		padding: var(--editor-pad);
		border-radius: inherit;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		overflow: hidden;
	}
	/* the words stay readable on any capture: the see-through panel ground fades in under them */
	.captured .open::before {
		content: '';
		position: absolute;
		inset: 45% 0 0;
		border-radius: inherit;
		background: linear-gradient(to top, var(--tool-panel), transparent);
		text-shadow: var(--tool-halo);
		pointer-events: none;
		z-index: 1;
	}
	/* the hover ground lies over the capture */
	.open::after {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		pointer-events: none;
		z-index: 1;
	}
	@media (hover: hover) {
		.open:hover::after {
			background: var(--kit-workspace-card-hover-ground);
		}
	}
	/* the focus ring: Tab, Shift Tab and the arrows move it */
	.open:focus {
		outline: none;
	}
	.ws-card:has(.open:focus-visible),
	.ws-card:has(.open[data-ring]:focus) {
		outline: 2px solid var(--tool-text);
		outline-offset: 1px;
	}
	.placeholder {
		display: flex;
		align-items: center;
		gap: var(--editor-gap);
		/* above the words, the card's upper part */
		margin-bottom: 2.5rem;
	}
	.front .placeholder {
		color: var(--tool-text);
	}
	.tag {
		position: absolute;
		z-index: 2;
		top: calc(var(--editor-pad) / 2);
		left: var(--editor-pad);
		color: var(--tool-text);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.words {
		position: relative;
		z-index: 2;
		display: flex;
		flex-direction: column;
		gap: calc(var(--editor-pad) / 4);
		min-width: 0;
	}
	.word {
		font-size: var(--editor-title);
	}
	.word,
	.quiet {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.quiet {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.captured .quiet {
		color: var(--tool-text);
	}
	/* the chip lies on the card; the card under it takes the click */
	.deck {
		position: absolute;
		z-index: 2;
		right: var(--editor-pad);
		bottom: var(--editor-pad);
		pointer-events: none;
	}
	.ws-card:has(.deck) .words {
		padding-right: 4rem;
	}
	.close {
		position: absolute;
		top: calc(var(--editor-pad) / 4);
		right: calc(var(--editor-pad) / 4);
	}
</style>
