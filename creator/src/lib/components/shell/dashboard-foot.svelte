<script lang="ts">
	import { onMount } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { Button, Menu, MenuHead, MenuItem } from '@scutoons/ui/editor';
	import { canUseApp } from '@scutoons/auth';
	import { ADMIN_URL, EMBASSY_URL, VIEWER_URL } from '$lib/config.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The dashboard's one quiet line at the bottom (crafts-v2.html `dash`): the account · Settings · Docs · help and the other
	 * apps · the build; the keys at the right end.
	 */
	let { reader }: { reader: string | null } = $props();

	const session = getCreatorSession();
	const roles = $derived(session.roles);
	const who = $derived(session.user?.email ?? '');

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
		['Ctrl ,', () => m['shell.key_settings']()],
		['Alt V', () => m['shell.key_views']()],
		['Ctrl Tab', () => m['shell.key_tab_next']()],
		['Ctrl Shift Tab', () => m['shell.key_tab_prev']()],
		['Ctrl W', () => m['shell.key_tab_close']()],
		['Ctrl Z / Ctrl Y', () => m['shell.key_undo']()],
		['Alt ↑', () => m['shell.key_up']()],
		['Ctrl Space', () => m['shell.key_full']()]
	];
</script>

<footer class="foot" data-testid="dash-foot">
	<span class="who" title={roles.join(' · ')}><Icon name="user" size="1rem" />{who}</span>
	<span class="sep" aria-hidden="true"></span>
	<Button variant="quiet" onclick={() => shell.open('settings')} data-testid="menu-settings">{m['shell.settings_menu']()}</Button>
	<span class="sep" aria-hidden="true"></span>
	<Button variant="quiet" href="/docs" target="_blank" rel="noopener" outward>{m['shell.docs']()}</Button>
	<Menu label={m['shell.help']()} icon="help-circle" align="start" side="above" width="22rem" testid="dash-help">
		{#snippet children(close)}
			<MenuItem icon="book-open" label={m['shell.guide']()} href="/guide" onclick={close} />
			<MenuHead label={m['shell.shortcuts']()} />
			{#each SHORTCUTS as [keys, what] (keys)}<MenuItem label={what()} kbd={keys} />{/each}
		{/snippet}
	</Menu>
	<Menu label={m['shell.go_to']()} icon="external-link" align="start" side="above" width="19rem" testid="dash-go-to">
		{#snippet children()}
			<MenuItem icon="book-open" label={reader ? m['shell.reader_this']() : m['creator.reader']()} href={reader ?? VIEWER_URL} external />
			<MenuItem icon="radio" label={m['shell.deck']()} href="{VIEWER_URL}/live/deck" external />
			{#if canUseApp('embassy', roles)}<MenuItem icon="mail" label="Embassy" href={EMBASSY_URL} external />{/if}
			{#if canUseApp('admin', roles)}<MenuItem icon="shield" label="Admin" href={ADMIN_URL} external />{/if}
		{/snippet}
	</Menu>
	<span class="sep" aria-hidden="true"></span>
	<span class="build" title={m['shell.version']()} data-testid="menu-build">{build}</span>
	<span class="keys">{m['shell.dash_keys']()}</span>
</footer>

<style>
	.foot {
		display: flex;
		align-items: center;
		gap: calc(var(--kit-gap) / 2);
		min-width: 0;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		text-shadow: var(--tool-halo);
	}
	.who {
		display: inline-flex;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-width: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-text);
	}
	.sep {
		align-self: center;
		height: 1rem;
		border-left: var(--editor-edge) solid var(--tool-line);
	}
	.build {
		padding: 0 calc(var(--editor-pad) / 2);
		font-family: var(--font-mono);
	}
	.keys {
		margin-left: auto;
		white-space: nowrap;
	}
</style>
