<script lang="ts">
	import { onMount } from 'svelte';
	import { Icon, clickOutside } from '@scutoons/ui';
	import { canUseApp } from '@scutoons/auth';
	import { ADMIN_URL, EMBASSY_URL, VIEWER_URL } from '$lib/config.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import BarFind from '$lib/components/shell/bar-find.svelte';
	import ViewsMenu from '$lib/components/shell/views-menu.svelte';

	/**
	 * The main menu (☰, far right; the deck's bar.rs MENU): Home first, then open, search and the Views, Settings, Docs,
	 * help and the other apps, the build at the end. Nothing to save – everything saves itself; ↗ leaves for the browser.
	 * `reader` is where the open item is read (null: the reader's start).
	 */
	let { reader }: { reader: string | null } = $props();

	const session = getCreatorSession();
	let open = $state(false);
	let sub = $state<'search' | 'views' | 'help' | 'goto' | null>(null);
	const roles = $derived(session.roles);

	// the build this is (version.json beside the app; a dev server has none)
	let build = $state('dev');
	onMount(() => {
		fetch('/version.json', { cache: 'no-store' })
			.then((r) => (r.ok ? r.json() : null))
			.then((v) => {
				if (v?.version) build = `${v.version}${v.git_sha && v.git_sha !== 'dev' ? ` · ${String(v.git_sha).slice(0, 7)}` : ''}`;
			})
			.catch(() => {});
	});

	function shut() {
		open = false;
		sub = null;
	}
	function run(action: () => void) {
		shut();
		action();
	}
	const toggle = (name: NonNullable<typeof sub>) => (sub = sub === name ? null : name);
	const SHORTCUTS: [string, () => string][] = [
		['Ctrl K', () => m['shell.key_find']()],
		['Ctrl O', () => m['shell.key_open']()],
		['Ctrl ,', () => m['shell.key_settings']()],
		['Ctrl Tab', () => m['shell.key_tab_next']()],
		['Ctrl Shift Tab', () => m['shell.key_tab_prev']()],
		['Ctrl W', () => m['shell.key_tab_close']()],
		['Ctrl Z / Ctrl Y', () => m['shell.key_undo']()],
		['Alt ↑', () => m['shell.key_up']()]
	];
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && shut()} />

<div class="main-menu" {@attach clickOutside(shut)}>
	<button type="button" class="round" aria-expanded={open} aria-haspopup="menu" aria-label={m['shell.menu']()} title={m['shell.menu']()} onclick={() => ((open = !open), (sub = null))} data-testid="menu-button">
		<Icon name="menu" size="1rem" />
	</button>
	{#if open}
		<div class="menu" role="menu" data-testid="main-menu">
			<a role="menuitem" href="/" aria-current={shell.state.front === null ? 'page' : undefined} onclick={shut} data-testid="menu-home"><Icon name="home" size="1rem" />{m['shell.dashboard']()}</a>
			<hr />
			<button type="button" role="menuitem" onclick={() => run(() => shell.open('library'))}><Icon name="folder" size="1rem" />{m['shell.open_menu']()}<kbd>Ctrl O</kbd></button>
			<button type="button" role="menuitem" aria-expanded={sub === 'search'} onclick={() => toggle('search')} data-testid="menu-search"><Icon name="search" size="1rem" />{m['shell.search']()}<kbd>▸</kbd></button>
			<button type="button" role="menuitem" aria-expanded={sub === 'views'} onclick={() => toggle('views')} data-testid="menu-views"><Icon name="layout" size="1rem" />{m['shell.views_menu']()}<kbd>▸</kbd></button>
			<hr />
			<button type="button" role="menuitem" onclick={() => run(() => shell.open('settings'))} data-testid="menu-settings"><Icon name="settings" size="1rem" />{m['shell.settings_menu']()}<kbd>Ctrl ,</kbd></button>
			<a role="menuitem" href="/docs" target="_blank" rel="noopener" onclick={shut}><Icon name="book" size="1rem" />{m['shell.docs']()}<kbd>↗</kbd></a>
			<button type="button" role="menuitem" aria-expanded={sub === 'help'} onclick={() => toggle('help')}><Icon name="help-circle" size="1rem" />{m['shell.help']()}<kbd>▸</kbd></button>
			<button type="button" role="menuitem" aria-expanded={sub === 'goto'} onclick={() => toggle('goto')}><Icon name="external-link" size="1rem" />{m['shell.go_to']()}<kbd>▸</kbd></button>
			<hr />
			<span class="line muted" title={m['shell.version']()} data-testid="menu-build">{m['shell.build']()}<kbd>{build}</kbd></span>
		</div>
		{#if sub}
			<div class="menu sub" class:wide={sub === 'search' || sub === 'views'} class:open-ended={sub === 'search'} role="menu" data-testid="menu-{sub}">
				{#if sub === 'search'}
					<BarFind focus onpick={shut} />
				{:else if sub === 'views'}
					<ViewsMenu onclose={shut} />
				{:else if sub === 'help'}
					<a role="menuitem" href="/guide" onclick={shut}><Icon name="book-open" size="1rem" />{m['shell.guide']()}</a>
					<span class="head">{m['shell.shortcuts']()}</span>
					{#each SHORTCUTS as [keys, what] (keys)}<span class="line">{what()}<kbd>{keys}</kbd></span>{/each}
				{:else}
					<a role="menuitem" href={reader ?? VIEWER_URL} target="_blank" rel="noopener"><Icon name="book-open" size="1rem" />{reader ? m['shell.reader_this']() : m['creator.reader']()}<kbd>↗</kbd></a>
					<a role="menuitem" href="{VIEWER_URL}/live/deck" target="_blank" rel="noopener"><Icon name="radio" size="1rem" />{m['shell.deck']()}<kbd>↗</kbd></a>
					{#if canUseApp('embassy', roles)}<a role="menuitem" href={EMBASSY_URL} target="_blank" rel="noopener"><Icon name="mail" size="1rem" />Embassy<kbd>↗</kbd></a>{/if}
					{#if canUseApp('admin', roles)}<a role="menuitem" href={ADMIN_URL} target="_blank" rel="noopener"><Icon name="shield" size="1rem" />Admin<kbd>↗</kbd></a>{/if}
				{/if}
			</div>
		{/if}
	{/if}
</div>

<style>
	.main-menu {
		position: relative;
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
	.round:hover,
	.round[aria-expanded='true'] {
		background: var(--editor-hover);
	}
	/* far right: the menu opens leftwards, a sub menu left of it */
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.65rem);
		right: -0.2rem;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		width: 17rem;
		max-height: calc(100dvh - var(--shell-top) - var(--status-room));
		padding: 0.3rem;
		overflow: auto;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		color: var(--editor-text);
	}
	.menu.sub {
		right: 17.3rem;
		width: 19rem;
	}
	.menu.sub.wide {
		width: 24rem;
	}
	/* the search's found rows hang below its field */
	.menu.sub.open-ended {
		overflow: visible;
	}
	.menu :global(button.line),
	.menu button,
	.menu a,
	.menu :global(.line) {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: flex-start;
		gap: 0.6rem;
		min-height: var(--control-height);
		padding: 0 0.6rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: var(--editor-text);
		text-align: left;
		text-decoration: none;
	}
	.menu button:hover:not(:disabled),
	.menu a:hover,
	.menu button[aria-expanded='true'],
	.menu a[aria-current='page'] {
		background: var(--editor-hover);
	}
	.menu button:disabled {
		color: var(--editor-muted);
	}
	kbd {
		margin-left: auto;
		color: var(--editor-muted);
		font-family: var(--font-mono);
		font-size: var(--editor-small);
	}
	.menu :global(.head) {
		padding: 0.4rem 0.6rem 0.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.muted {
		color: var(--editor-muted);
	}
	.menu :global(hr) {
		width: 100%;
		margin: 0.25rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--editor-line);
	}
</style>
