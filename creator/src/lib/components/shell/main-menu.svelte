<script lang="ts">
	import { Icon, clickOutside } from '@scutoons/ui';
	import { canUseApp } from '@scutoons/auth';
	import { ADMIN_URL, EMBASSY_URL, VIEWER_URL } from '$lib/config.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The main menu (☰; research/redesign/PLAN.md §2.5, the deck's bar.rs MENU): the deck's lines and the creator's,
	 * then the open workspace's panels that can close (ticked = open). Nothing to save – everything saves itself; ↗ leaves
	 * for the browser. `reader` is where the open item is read (null: the reader's start).
	 */
	let { reader }: { reader: string | null } = $props();

	const session = getCreatorSession();
	let open = $state(false);
	let sub = $state<'language' | 'help' | 'goto' | 'account' | null>(null);
	const roles = $derived(session.roles);
	const languages = $derived(textLanguage.page.languages);
	const current = $derived(textLanguage.current(languages, textLanguage.page.source));

	function run(action: () => void) {
		open = false;
		sub = null;
		action();
	}
	const SHORTCUTS: [string, () => string][] = [
		['Ctrl K', () => m['shell.key_find']()],
		['Ctrl O', () => m['shell.key_open']()],
		['Ctrl ,', () => m['shell.key_settings']()],
		['Ctrl S', () => m['shell.key_save']()],
		['Ctrl Z / Ctrl Y', () => m['shell.key_undo']()],
		['Alt ↑', () => m['shell.key_up']()]
	];
</script>

<div class="main-menu" {@attach clickOutside(() => ((open = false), (sub = null)))}>
	<button type="button" class="round" aria-expanded={open} aria-haspopup="menu" aria-label={m['shell.menu']()} title={m['shell.menu']()} onclick={() => ((open = !open), (sub = null))} data-testid="menu-button">
		<Icon name="menu" size="1rem" />
	</button>
	{#if open}
		<div class="menu" role="menu" data-testid="main-menu">
			<button type="button" role="menuitem" onclick={() => run(() => shell.open('library'))}><Icon name="folder" size="1rem" />{m['shell.open_menu']()}<kbd>Ctrl O</kbd></button>
			<button type="button" role="menuitem" disabled title={m['shell.later']()}><Icon name="plus" size="1rem" />{m['shell.new_menu']()}<kbd>{m['shell.later']()}</kbd></button>
			<button type="button" role="menuitem" disabled={!shell.doc} onclick={() => run(() => shell.open('document'))}><Icon name="upload-cloud" size="1rem" />{m['shell.publish_menu']()}<kbd>{m['shell.this_item']()}</kbd></button>
			<hr />
			<button type="button" role="menuitem" onclick={() => run(() => shell.open('settings'))} data-testid="menu-settings"><Icon name="settings" size="1rem" />{m['shell.settings_menu']()}<kbd>Ctrl ,</kbd></button>
			{#if languages.length > 1}
				<button type="button" role="menuitem" aria-expanded={sub === 'language'} onclick={() => (sub = sub === 'language' ? null : 'language')}><Icon name="type" size="1rem" />{m['shell.text_language']()}<kbd>{current.toUpperCase()} ▸</kbd></button>
			{/if}
			<button type="button" role="menuitem" aria-expanded={sub === 'help'} onclick={() => (sub = sub === 'help' ? null : 'help')}><Icon name="help-circle" size="1rem" />{m['shell.help']()}<kbd>▸</kbd></button>
			<button type="button" role="menuitem" aria-expanded={sub === 'goto'} onclick={() => (sub = sub === 'goto' ? null : 'goto')}><Icon name="external-link" size="1rem" />{m['shell.go_to']()}<kbd>▸</kbd></button>
			<hr />
			<button type="button" role="menuitem" aria-expanded={sub === 'account'} onclick={() => (sub = sub === 'account' ? null : 'account')}><Icon name="user" size="1rem" />{m['shell.account']()}<kbd>▸</kbd></button>
			{#if shell.panels.length}
				<hr />
				<span class="head">{m['shell.panels_here']()}</span>
				{#each shell.panels as p (p.id)}
					<button type="button" role="menuitemcheckbox" aria-checked={p.open} onclick={() => (shell.toggled = p.id)} data-testid="menu-panel-{p.id}">
						<span class="tick" class:on={p.open} aria-hidden="true">{#if p.open}<Icon name="check" size="0.8rem" />{/if}</span>{p.name}
					</button>
				{/each}
			{/if}
		</div>
		{#if sub}
			<div class="menu sub" role="menu" data-testid="menu-{sub}">
				{#if sub === 'language'}
					{#each languages as l (l)}
						<button type="button" role="menuitemradio" aria-checked={l === current} onclick={() => run(() => textLanguage.set(l === textLanguage.page.source ? '' : l))}>{l.toUpperCase()}</button>
					{/each}
				{:else if sub === 'help'}
					<a role="menuitem" href="/guide" onclick={() => run(() => {})}><Icon name="book-open" size="1rem" />{m['shell.guide']()}</a>
					<a role="menuitem" href="/docs" target="_blank" rel="noopener"><Icon name="external-link" size="1rem" />{m['shell.docs']()}<kbd>↗</kbd></a>
					<span class="head">{m['shell.shortcuts']()}</span>
					{#each SHORTCUTS as [keys, what] (keys)}<span class="line">{what()}<kbd>{keys}</kbd></span>{/each}
				{:else if sub === 'goto'}
					<a role="menuitem" href={reader ?? VIEWER_URL} target="_blank" rel="noopener"><Icon name="book-open" size="1rem" />{reader ? m['shell.reader_this']() : m['creator.reader']()}<kbd>↗</kbd></a>
					<a role="menuitem" href="{VIEWER_URL}/live/deck" target="_blank" rel="noopener"><Icon name="radio" size="1rem" />{m['shell.deck']()}<kbd>↗</kbd></a>
					{#if canUseApp('embassy', roles)}<a role="menuitem" href={EMBASSY_URL} target="_blank" rel="noopener"><Icon name="mail" size="1rem" />Embassy<kbd>↗</kbd></a>{/if}
					{#if canUseApp('admin', roles)}<a role="menuitem" href={ADMIN_URL} target="_blank" rel="noopener"><Icon name="shield" size="1rem" />Admin<kbd>↗</kbd></a>{/if}
				{:else}
					<span class="line">{session.user?.email ?? ''}</span>
					<span class="line muted">{roles.join(' · ')}</span>
					<button type="button" role="menuitem" onclick={() => run(() => void session.logout())} data-testid="sign-out"><Icon name="log-out" size="1rem" />{m['creator.sign_out']()}</button>
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
		min-width: var(--editor-row);
		padding: 0;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 50%;
		background: var(--editor-surface);
		color: var(--editor-text);
	}
	.round:hover,
	.round[aria-expanded='true'] {
		background: var(--editor-hover);
	}
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		left: 0;
		display: flex;
		flex-direction: column;
		width: 17rem;
		padding: 0.3rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
		box-shadow: 0 0.4rem 1.2rem rgb(0 0 0 / 0.4);
	}
	.menu.sub {
		left: 18rem;
	}
	.menu button,
	.menu a,
	.line {
		display: flex;
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
	.menu button[aria-expanded='true'] {
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
	.head {
		padding: 0.4rem 0.6rem 0.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.tick {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1rem;
		height: 1rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 0.2rem;
	}
	.tick.on {
		border-color: var(--editor-primary-line);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
	}
	.muted {
		color: var(--editor-muted);
	}
	hr {
		width: 100%;
		margin: 0.25rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--editor-line);
	}
</style>
