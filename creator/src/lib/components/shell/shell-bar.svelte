<script lang="ts">
	import { IconButton, Menu, Pill } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import TabStrip from '$lib/components/shell/tab-strip.svelte';
	import WorkspaceTabs from '$lib/components/shell/workspace-tabs.svelte';

	/**
	 * One bar for the creator and the deck editor (research/redesign/lap/crafts-v2.html, view `bars`; deck/editor/src/bar.rs):
	 * the open items' tabs at the left, the workspaces in the middle, icons only at the right end – ↶ ↷, Publish (only while
	 * something in its scope is not live), the account (the user menu, as a swipe up from the bottom) and the menu, far right: it lays the dashboard over everything and
	 * takes it away again. Every part is a kit Pill, half a gap apart; the icons at the right stand bare, without the pill's ground (owner 2026-11-24: room).
	 */
	const session = getCreatorSession();
	const doc = $derived(shell.doc);
	const who = $derived(session.user?.email ?? '');
	// a page that cannot tell (unpublished not given) keeps Publish; one that says nothing waits hides it
	const publishing = $derived(!!doc?.publish && doc.unpublished !== false);
</script>

<header class="shell-bar" data-testid="creator-bar" data-control-exempt="the bar's pills: deck-kit bar.rs's inset height">
	<TabStrip />
	<span class="middle"><WorkspaceTabs /></span>
	{#if doc?.undo}
		<Pill bare label={m['history.undo']()}>
			<IconButton label={m['history.undo']()} title={m['history.undo_hint']()} icon="rotate-ccw" onclick={() => doc?.undo?.()} data-testid="bar-undo" />
			<IconButton label={m['history.redo']()} title={m['history.redo_hint']()} icon="rotate-cw" onclick={() => doc?.redo?.()} data-testid="bar-redo" />
		</Pill>
	{/if}
	<Pill bare>
		{#if publishing}
			<Menu label={m['shell.panel_publish']()} icon="upload-cloud" about={m['shell.publish_bar']()} testid="bar-publish" dot={!!doc?.unpublished} width="19rem">
				{#snippet children()}
					{#if doc?.publish}{@render doc.publish()}{/if}
				{/snippet}
			</Menu>
		{/if}
		<IconButton label={m['shell.account_menu']({ who })} title={m['shell.user_menu']()} icon="user" pressed={shell.userMenu} aria-expanded={shell.userMenu} onclick={() => (shell.userMenu = !shell.userMenu)} data-testid="bar-account" />
		<IconButton label={m['shell.dashboard']()} title="{m['shell.dashboard']()} (Esc)" icon="menu" pressed={!!shell.dashboard} aria-expanded={!!shell.dashboard} onclick={() => (shell.dashboard = shell.dashboard ? false : 'cards')} data-testid="menu-button" />
	</Pill>
</header>

<style>
	/* the bar's row lies on the stage a margin from the top (bar.rs `margin`): only its pills take the pointer */
	.shell-bar {
		position: fixed;
		z-index: 50;
		top: 0;
		left: 0;
		right: 0;
		display: flex;
		flex-wrap: nowrap;
		align-items: center;
		gap: var(--kit-gap);
		box-sizing: border-box;
		height: var(--shell-top);
		padding: 0 var(--editor-gap);
		color: var(--tool-text);
		font-family: var(--editor-sans);
		pointer-events: none;
	}
	.shell-bar > :global(*) {
		pointer-events: auto;
	}
	.middle {
		display: flex;
		flex: 1 1 0;
		justify-content: center;
		min-width: 0;
		pointer-events: none;
	}
	.middle > :global(*) {
		pointer-events: auto;
	}
</style>
