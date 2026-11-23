<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import BarMenu from '$lib/components/shell/bar-menu.svelte';
	import MainMenu from '$lib/components/shell/main-menu.svelte';
	import TabStrip from '$lib/components/shell/tab-strip.svelte';
	import WorkspaceTabs from '$lib/components/shell/workspace-tabs.svelte';

	/**
	 * One bar for the creator and the deck editor (research/redesign/lap/crafts-v2.html, view `bars`): the open items' tabs
	 * at the left, the workspaces in the middle, icons only at the right end – ↶ ↷, Publish (only while something in its
	 * scope is not live), the account and the menu, far right. Home, Search, Views, Settings, Docs and the build are the
	 * menu's; History is a view.
	 */
	let { reader }: { reader: string | null } = $props();

	const session = getCreatorSession();
	const doc = $derived(shell.doc);
	const who = $derived(session.user?.email ?? '');
	// a page that cannot tell (unpublished not given) keeps Publish; one that says nothing waits hides it
	const publishing = $derived(!!doc?.publish && doc.unpublished !== false);
</script>

<header class="shell-bar" data-testid="creator-bar">
	<TabStrip />
	<span class="middle"><WorkspaceTabs /></span>
	{#if doc?.undo}
		<span class="pill pair">
			<button type="button" class="round" onclick={() => doc?.undo?.()} aria-label={m['history.undo']()} title={m['history.undo_hint']()} data-testid="bar-undo"><Icon name="rotate-ccw" size="1rem" /></button>
			<button type="button" class="round" onclick={() => doc?.redo?.()} aria-label={m['history.redo']()} title={m['history.redo_hint']()} data-testid="bar-redo"><Icon name="rotate-cw" size="1rem" /></button>
		</span>
	{/if}
	<span class="pill pair end">
		{#if publishing}
			<BarMenu label={m['shell.panel_publish']()} icon="upload-cloud" about={m['shell.publish_bar']()} testid="bar-publish" dot={!!doc?.unpublished}>
				{#snippet children()}
					{#if doc?.publish}{@render doc.publish()}{/if}
				{/snippet}
			</BarMenu>
		{/if}
		<BarMenu label={m['shell.account_menu']({ who })} icon="user" testid="bar-account">
			{#snippet children(close)}
				<span class="line">{who}</span>
				<span class="line muted">{session.roles.join(' · ')}</span>
				<hr />
				<button type="button" class="line" onclick={() => (close(), void session.logout())} data-testid="sign-out"><Icon name="log-out" size="1rem" />{m['creator.sign_out']()}</button>
			{/snippet}
		</BarMenu>
		<MainMenu {reader} />
	</span>
</header>

<style>
	/* the bar's row lies on the stage: only its pills take the pointer */
	.shell-bar {
		position: fixed;
		z-index: 50;
		top: 0;
		left: 0;
		right: 0;
		display: flex;
		flex-wrap: nowrap;
		align-items: center;
		gap: 0.45rem;
		box-sizing: border-box;
		height: var(--shell-top);
		padding: 0 1rem;
		color: var(--editor-text);
		font-family: var(--editor-sans);
		pointer-events: none;
	}
	.shell-bar > :global(*) {
		pointer-events: auto;
	}
	.pill {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		min-height: var(--control-height);
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.2rem;
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
		color: var(--editor-text);
	}
	.pair {
		gap: 0.15rem;
		padding: 0 0.2rem;
	}
	.round {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: calc(var(--control-height) - 0.4rem);
		min-height: calc(var(--control-height) - 0.4rem);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: var(--editor-text);
	}
	.round:hover {
		background: var(--editor-hover);
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
	.muted {
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
</style>
