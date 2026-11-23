<script lang="ts">
	import { onMount } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { Menu, MenuHead, MenuItem } from '@scutoons/ui/editor';
	import { canUseApp } from '@scutoons/auth';
	import { ADMIN_URL, EMBASSY_URL, VIEWER_URL } from '$lib/config.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { tabSpecs } from '$lib/shell/panels.ts';
	import { shell, type MenuSub } from '$lib/shell/shell.svelte.ts';
	import { saveWorkspace } from '$lib/shell/workspace-actions.ts';
	import ViewsMenu from '$lib/components/shell/views-menu.svelte';
	import TextLanguageSwitch from '$lib/components/text/text-language-switch.svelte';

	/**
	 * The ☰ menu, far right in the bar (lap 2 A2): Views – every view of the front workspace with its four states, only
	 * here – and Save workspace; Search, Settings, Docs, Help and Go to; the text language; the account and the build.
	 * Views, Help and Go to open their own panel beside the menu.
	 */
	let { reader = null }: { reader?: string | null } = $props();

	const session = getCreatorSession();
	const roles = $derived(session?.roles ?? []);
	const who = $derived(session?.user?.email ?? '');
	const specs = tabSpecs();
	// the views shown now (in a panel, on the stage, full), by name: the Views line's end
	const shown = $derived(
		specs
			.filter((t) => t.id !== 'settings' && shell.available.has(t.id) && (shell.floats?.stateOf(t.id) ?? 'hidden') !== 'hidden')
			.map((t) => t.name)
			.join(' · ')
	);
	const languages = $derived(textLanguage.page.languages);
	const embassy = $derived(canUseApp('embassy', roles));
	const admin = $derived(canUseApp('admin', roles));
	const goTo = $derived([m['creator.reader'](), m['shell.deck_word'](), ...(embassy ? ['Embassy'] : []), ...(admin ? ['Admin'] : [])].join(' · '));

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

	const SHORTCUTS: [string, () => string][] = [
		['Alt Space · Ctrl K', () => m['shell.key_find']()],
		['Ctrl S', () => m['shell.ws_save']()],
		['Ctrl ,', () => m['shell.key_settings']()],
		['Alt V', () => m['shell.key_views']()],
		['Ctrl Tab', () => m['shell.key_tab_next']()],
		['Ctrl Shift Tab', () => m['shell.key_tab_prev']()],
		['Ctrl W', () => m['shell.key_tab_close']()],
		['Ctrl Z / Ctrl Y', () => m['shell.key_undo']()],
		['Alt ↑', () => m['shell.key_up']()],
		['Ctrl Space', () => m['shell.key_full']()]
	];
	const WIDTHS: Record<MenuSub, string> = { views: '26rem', help: '22rem', go: '19rem' };

	const toggle = (sub: MenuSub) => () => (shell.menuSub = shell.menuSub === sub ? null : sub);
	const then = (close: () => void, action: () => unknown) => () => {
		close();
		void action();
	};
</script>

{#snippet line(label: string, meta: string, sub: MenuSub)}
	<span class="line"><span class="name">{label}</span><span class="meta">{meta}</span><Icon name={shell.menuSub === sub ? 'chevron-left' : 'chevron-right'} size="1rem" strokeWidth={2} /></span>
{/snippet}

{#snippet beside(close: () => void)}
	{#if shell.menuSub === 'views'}
		<ViewsMenu onclose={close} />
	{:else if shell.menuSub === 'help'}
		<MenuItem icon="book-open" label={m['shell.guide']()} href="/guide" onclick={close} />
		<MenuHead label={m['shell.shortcuts']()} />
		{#each SHORTCUTS as [keys, what] (keys)}<MenuItem label={what()} kbd={keys} />{/each}
	{:else if shell.menuSub === 'go'}
		<MenuItem icon="book-open" label={reader ? m['shell.reader_this']() : m['creator.reader']()} href={reader ?? VIEWER_URL} external onclick={close} />
		<MenuItem icon="radio" label={m['shell.deck']()} href="{VIEWER_URL}/live/deck" external onclick={close} />
		{#if embassy}<MenuItem icon="mail" label="Embassy" href={EMBASSY_URL} external onclick={close} />{/if}
		{#if admin}<MenuItem icon="shield" label="Admin" href={ADMIN_URL} external onclick={close} />{/if}
	{/if}
{/snippet}

<Menu
	label={m['shell.menu']()}
	icon="menu"
	width="21rem"
	bind:open={shell.menu}
	onclose={() => (shell.menuSub = null)}
	beside={shell.menuSub ? beside : undefined}
	besideWidth={shell.menuSub ? WIDTHS[shell.menuSub] : undefined}
	testid="menu-button"
	besideTestid="menu-beside"
>
	{#snippet children(close)}
		<MenuItem icon="layout" expanded={shell.menuSub === 'views'} onclick={toggle('views')} data-testid="menu-views">{@render line(m['shell.views_menu'](), shown, 'views')}</MenuItem>
		<MenuItem icon="save" label={m['shell.ws_save']()} kbd="Ctrl S" onclick={then(close, saveWorkspace)} data-testid="menu-save" />
		<hr />
		<MenuItem icon="search" label={m['shell.search_menu']()} kbd="Alt Space" onclick={then(close, () => shell.toggleLauncher(true))} data-testid="menu-search" />
		<MenuItem icon="settings" label={m['shell.settings']()} onclick={then(close, () => shell.open('settings'))} data-testid="menu-settings" />
		<MenuItem icon="book" label="{m['shell.docs']()} ↗" href="/docs" target="_blank" rel="noopener" onclick={close} data-testid="menu-docs" />
		<MenuItem icon="help-circle" expanded={shell.menuSub === 'help'} onclick={toggle('help')} data-testid="menu-help">{@render line(m['shell.help'](), m['shell.help_lines'](), 'help')}</MenuItem>
		<MenuItem icon="external-link" expanded={shell.menuSub === 'go'} onclick={toggle('go')} data-testid="menu-go-to">{@render line(m['shell.go_to'](), goTo, 'go')}</MenuItem>
		{#if languages.length > 1}
			<hr />
			<MenuItem icon="globe" data-testid="menu-text-language">
				<span class="line"><span class="name">{m['shell.text_language']()}</span><span class="end"><TextLanguageSwitch {languages} source={textLanguage.page.source} /></span></span>
			</MenuItem>
		{/if}
		<hr />
		<MenuItem icon="user" label={who} title={roles.join(' · ')} data-testid="menu-who" />
		<MenuItem muted title={m['shell.version']()} data-testid="menu-build"><span class="build">{build}</span></MenuItem>
	{/snippet}
</Menu>

<style>
	/* a line's words, what it holds at its end, the panel's chevron */
	.line {
		display: flex;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-width: 0;
	}
	.name {
		flex: none;
	}
	.meta {
		flex: 1 1 auto;
		text-align: right;
		min-width: 0;
		overflow: hidden;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		text-overflow: ellipsis;
	}
	.end {
		margin-left: auto;
	}
	.build {
		font-family: var(--font-mono);
		font-size: var(--editor-small);
	}
</style>
