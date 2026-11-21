<script lang="ts">
	import { onMount } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import type { Crumb } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import BarFind from '$lib/components/shell/bar-find.svelte';
	import BarMenu from '$lib/components/shell/bar-menu.svelte';
	import MainMenu from '$lib/components/shell/main-menu.svelte';
	import TabStrip from '$lib/components/shell/tab-strip.svelte';
	import ViewsMenu from '$lib/components/shell/views-menu.svelte';

	/**
	 * The bar: pills on the stage, ordered like the deck editor's (deck/editor/src/bar.rs; research/redesign/panels.html) –
	 * ☰ · ⌂ Home · the open items' tabs (＋ = the Library) on the left; in the middle Find on Home, the item's path inside an
	 * item; on the right ↶ ↷, Views, History and Publish (menus under their pills, as OSCT's), the account and the
	 * version. The text language and Day / Night live in Settings only; the save state is the status bar's.
	 */
	let { path, reader }: { path: Crumb[]; reader: string | null } = $props();

	const session = getCreatorSession();
	const doc = $derived(shell.doc);

	// the build this is (version.json beside the app; a dev server has none)
	let version = $state('dev');
	onMount(() => {
		fetch('/version.json', { cache: 'no-store' })
			.then((r) => (r.ok ? r.json() : null))
			.then((v) => {
				if (v?.version) version = `${v.version}${v.git_sha && v.git_sha !== 'dev' ? ` · ${String(v.git_sha).slice(0, 7)}` : ''}`;
			})
			.catch(() => {});
	});
</script>

<header class="shell-bar" data-testid="creator-bar">
	<span class="pill pair"><MainMenu {reader} /><a class="round" href="/" aria-current={shell.state.front === null ? 'page' : undefined} aria-label={m['shell.home']()} title={m['shell.home']()} data-testid="creator-brand"><Icon name="home" size="1rem" /></a></span>
	<TabStrip />
	<span class="middle">
		{#if shell.state.front === null}
			<BarFind />
		{:else if path.length}
			<nav class="pill path" aria-label={m['creator.breadcrumb']()} data-testid="bar-path">
				<Icon name={iconOf(shell.state.front)} size="1rem" />
				{#each path as step, i (step.href + i)}
					{#if i > 0}<Icon name="chevron-right" size="0.75rem" />{/if}
					{#if i < path.length - 1}<a class="step" href={step.href}>{step.label}</a>{:else}<span class="name">{step.label}</span>{/if}
				{/each}
			</nav>
		{/if}
	</span>
	{#if doc?.undo}
		<span class="pill pair">
			<button type="button" class="round" onclick={() => doc?.undo?.()} aria-label={m['history.undo']()} title={m['history.undo_hint']()} data-testid="bar-undo"><Icon name="rotate-ccw" size="1rem" /></button>
			<button type="button" class="round" onclick={() => doc?.redo?.()} aria-label={m['history.redo']()} title={m['history.redo_hint']()} data-testid="bar-redo"><Icon name="rotate-cw" size="1rem" /></button>
		</span>
	{/if}
	<ViewsMenu />
	<BarMenu label={m['shell.history_menu']()} icon="clock" about={m['shell.history_about']()} testid="bar-history" wide>
		{#snippet children()}
			{#if doc?.docs?.length}<HistoryPanel docs={doc.docs} />{:else}<p class="empty">{m['shell.history_none']()}</p>{/if}
		{/snippet}
	</BarMenu>
	<BarMenu label={m['shell.panel_publish']()} icon="upload-cloud" about={m['shell.publish_about']()} testid="bar-publish" dot={!!doc?.unpublished}>
		{#snippet children()}
			{#if doc?.publish}{@render doc.publish()}{:else}<p class="empty">{m['shell.publish_none']()}</p>{/if}
		{/snippet}
	</BarMenu>
	<span class="pill account" title={session.user?.email ?? ''}><Icon name="user" size="1rem" /><span class="who">{session.user?.email?.split('@')[0] ?? ''}</span></span>
	<span class="pill version" title={m['shell.version']()}>{version}</span>
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
	.round,
	.pill {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		box-sizing: border-box;
		min-height: var(--control-height);
		border: var(--editor-edge) solid var(--editor-line);
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
		color: var(--editor-text);
		text-decoration: none;
	}
	.round {
		min-width: var(--control-height);
		border-radius: 50%;
	}
	.pair .round,
	.pair :global(.round) {
		min-width: calc(var(--control-height) - 0.4rem);
		min-height: calc(var(--control-height) - 0.4rem);
		border: 0;
		padding: 0;
		background: transparent;
	}
	.pair .round:hover,
	.pair :global(.round:hover) {
		background: var(--editor-hover);
	}
	.round[aria-current='page'] {
		background: var(--editor-pressed) !important;
		color: var(--editor-pressed-text);
	}
	.pill {
		padding: 0 0.85rem;
		border-radius: 1.2rem;
		white-space: nowrap;
	}
	.pair {
		gap: 0.15rem;
		padding: 0 0.2rem;
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
	.path {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.step {
		flex: 0 1 auto;
		min-width: 2rem;
		overflow: hidden;
		text-overflow: ellipsis;
		color: inherit;
		text-decoration: none;
	}
	.step:hover {
		color: var(--editor-text);
		text-decoration: underline;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--editor-text);
	}
	.who {
		max-width: 8rem;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.version {
		color: var(--editor-muted);
		font-family: var(--font-mono);
		font-size: var(--editor-small);
	}
</style>
