<script lang="ts">
	import { CloseIcon, Icon, ToonAvatar } from '@scutoons/ui';
	import { IconButton, Menu } from '@scutoons/ui/editor';
	import { toonAsset } from '@scutoons/content';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf, workspaceOf } from '$lib/shell/address.ts';
	import type { WorkspaceCard } from '$lib/shell/dashboard.ts';
	import { tabSpecs } from '$lib/shell/panels.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { workspaceName } from '$lib/shell/workspace-names.ts';
	import ViewsMenu from '$lib/components/shell/views-menu.svelte';

	/**
	 * A workspace on the dashboard: what is open in it (the item's toon and its kind's icon as its picture, its name), ✕
	 * closes that item; the front one carries its Views line (the front workspace's Views). Without an item it says so.
	 */
	let {
		card,
		ring = false,
		views = $bindable(false),
		onopen,
		onclose,
		onfocus
	}: { card: WorkspaceCard; ring?: boolean; views?: boolean; onopen: (card: WorkspaceCard) => void; onclose: (address: string) => void; onfocus: () => void } = $props();

	const nav = getCreatorNav();
	const specs = tabSpecs();
	const name = $derived(card.name ?? workspaceName(card.kind));
	const item = $derived(card.shown);
	const toon = $derived(item ? (item.address.split(':')[1] ?? null) : null);
	const avatar = $derived.by(() => {
		const ws = item ? workspaceOf(item.address) : null;
		const t = toon && (ws === 'toon' || ws === 'chapter' || ws === 'storyboard') ? nav.toons.find((x) => x.id === toon) : undefined;
		return t?.avatar ? asset(toonAsset(t.id, t.avatar)) : null;
	});
	const more = $derived(card.open.length > 1 ? ` +${card.open.length - 1}` : '');
	// the front workspace's views with content, by name (its Views line)
	const viewNames = $derived(card.front ? specs.filter((t) => t.id !== 'settings' && shell.available.has(t.id)).map((t) => t.name).join(' · ') : '');
</script>

<div class="ws-card" class:front={card.front} class:empty={!item} data-testid="dash-card" data-kind={card.kind} data-workspace={card.id}>
	{#snippet inside()}
		<span class="picture" aria-hidden="true">
			{#if avatar}<ToonAvatar src={avatar} name={toon ?? ''} size="2.6rem" />{/if}
			<Icon name={item ? iconOf(item.address) : 'layout'} size="2rem" strokeWidth={1.25} />
		</span>
		{#if card.front}<span class="tag">{m['shell.dash_front']()}</span>{/if}
		<span class="word">{name}</span>
		<span class="quiet">{item ? `${item.label}${more}` : m['shell.dash_nothing_open']()}</span>
	{/snippet}
	{#if card.href}
		<a class="open" href={card.href} data-ring={ring || undefined} onclick={() => onopen(card)} {onfocus} data-testid="dash-card-open">{@render inside()}</a>
	{:else}
		<button type="button" class="open" data-ring={ring || undefined} onclick={() => onopen(card)} {onfocus} data-testid="dash-card-open">{@render inside()}</button>
	{/if}
	{#if item}
		<span class="close">
			<IconButton label={m['shell.close_tab']({ name: item.label })} onclick={() => onclose(item.address)} tabindex={-1} data-testid="dash-card-close"><CloseIcon size="0.8rem" stroke={1} /></IconButton>
		</span>
	{/if}
	{#if card.front && viewNames}
		<div class="views" data-testid="dash-views">
			<Menu label={m['shell.views_menu']()} icon="layout" align="start" width="28rem" bind:open={views} testid="dash-views-button">
				{#snippet children(close)}<ViewsMenu onclose={close} />{/snippet}
			</Menu>
			<span class="views-word">{m['shell.views_menu']()}</span>
			<span class="views-list">{viewNames}</span>
		</div>
	{/if}
</div>

<style>
	/* a card on the stage (the kit Card's ground, line and corner; deck-kit home.rs) */
	.ws-card {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		border: var(--editor-edge) solid var(--tool-line);
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
		display: flex;
		flex-direction: column;
		flex: 1;
		align-items: stretch;
		justify-content: flex-start;
		gap: calc(var(--editor-pad) / 4);
		min-width: 0;
		padding: 0 var(--editor-pad) var(--editor-pad);
		border-radius: var(--kit-radius-box);
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	@media (hover: hover) {
		.open:hover {
			background: var(--tool-hover);
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
	.picture {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--editor-gap);
		height: 5.5rem;
		color: var(--tool-muted);
	}
	.front .picture {
		color: var(--tool-text);
	}
	.tag {
		position: absolute;
		top: calc(var(--editor-pad) / 2);
		left: var(--editor-pad);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
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
	.close {
		position: absolute;
		top: calc(var(--editor-pad) / 4);
		right: calc(var(--editor-pad) / 4);
	}
	.views {
		display: flex;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-width: 0;
		margin: 0 calc(var(--editor-pad) / 2) calc(var(--editor-pad) / 2);
		border-top: var(--editor-edge) solid var(--tool-line);
	}
	.views-list {
		min-width: 0;
		margin-left: auto;
		overflow: hidden;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
