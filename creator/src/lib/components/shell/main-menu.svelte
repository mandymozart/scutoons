<script lang="ts">
	import { onMount } from 'svelte';
	import { Menu, MenuHead, MenuItem } from '@scutoons/ui/editor';
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
	const SUB_WIDTH = { search: '24rem', views: '28rem', help: '19rem', goto: '19rem' } as const;
	const toggle = (name: NonNullable<typeof sub>) => (sub = sub === name ? null : name);
	const SHORTCUTS: [string, () => string][] = [
		['Ctrl K', () => m['shell.key_find']()],
		['Ctrl O', () => m['shell.key_open']()],
		['Ctrl ,', () => m['shell.key_settings']()],
		['Ctrl Tab', () => m['shell.key_tab_next']()],
		['Ctrl Shift Tab', () => m['shell.key_tab_prev']()],
		['Ctrl W', () => m['shell.key_tab_close']()],
		['Ctrl Z / Ctrl Y', () => m['shell.key_undo']()],
		['Alt ↑', () => m['shell.key_up']()],
		['Ctrl Space', () => m['shell.key_full']()]
	];
</script>

<Menu label={m['shell.menu']()} icon="menu" bind:open onclose={() => (sub = null)} testid="menu-button" menuTestid="main-menu" beside={sub ? beside : undefined} besideWidth={sub ? SUB_WIDTH[sub] : undefined} besideOpen={sub === 'search'} besideTestid="menu-{sub}">
	{#snippet children()}
		<MenuItem icon="home" label={m['shell.dashboard']()} href="/" current={shell.state.front === null} onclick={shut} data-testid="menu-home" />
		<hr />
		<MenuItem icon="folder" label={m['shell.open_menu']()} kbd="Ctrl O" onclick={() => run(() => shell.open('library'))} />
		<MenuItem icon="search" label={m['shell.search']()} sub expanded={sub === 'search'} onclick={() => toggle('search')} data-testid="menu-search" />
		<MenuItem icon="layout" label={m['shell.views_menu']()} sub expanded={sub === 'views'} onclick={() => toggle('views')} data-testid="menu-views" />
		<hr />
		<MenuItem icon="settings" label={m['shell.settings_menu']()} kbd="Ctrl ," onclick={() => run(() => shell.open('settings'))} data-testid="menu-settings" />
		<MenuItem icon="book" label={m['shell.docs']()} href="/docs" external onclick={shut} />
		<MenuItem icon="help-circle" label={m['shell.help']()} sub expanded={sub === 'help'} onclick={() => toggle('help')} />
		<MenuItem icon="external-link" label={m['shell.go_to']()} sub expanded={sub === 'goto'} onclick={() => toggle('goto')} />
		<hr />
		<MenuItem label={m['shell.build']()} kbd={build} muted title={m['shell.version']()} data-testid="menu-build" />
	{/snippet}
</Menu>

{#snippet beside()}
	{#if sub === 'search'}
		<BarFind focus onpick={shut} />
	{:else if sub === 'views'}
		<ViewsMenu onclose={shut} />
	{:else if sub === 'help'}
		<MenuItem icon="book-open" label={m['shell.guide']()} href="/guide" onclick={shut} />
		<MenuHead label={m['shell.shortcuts']()} />
		{#each SHORTCUTS as [keys, what] (keys)}<MenuItem label={what()} kbd={keys} />{/each}
	{:else}
		<MenuItem icon="book-open" label={reader ? m['shell.reader_this']() : m['creator.reader']()} href={reader ?? VIEWER_URL} external />
		<MenuItem icon="radio" label={m['shell.deck']()} href="{VIEWER_URL}/live/deck" external />
		{#if canUseApp('embassy', roles)}<MenuItem icon="mail" label="Embassy" href={EMBASSY_URL} external />{/if}
		{#if canUseApp('admin', roles)}<MenuItem icon="shield" label="Admin" href={ADMIN_URL} external />{/if}
	{/if}
{/snippet}
