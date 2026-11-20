<script lang="ts">
	import { onMount } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import type { Crumb } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import TextLanguageSwitch from '$lib/components/text/text-language-switch.svelte';
	import MainMenu from '$lib/components/shell/main-menu.svelte';
	import TabStrip from '$lib/components/shell/tab-strip.svelte';

	/**
	 * The top bar, styled and ordered like the deck editor's (deck/editor/src/bar.rs; research/redesign/PLAN.md §2.4):
	 * pills in one row over the stage – ☰ · ⌂ Home · the open items' tabs (＋ = the Library) on the left; ↶ ↷ (unsaved
	 * edits, then the saved changes of this session), the save state, the text language (the one switch, RULES H1), the account and the version on the right. No workspace switch: the
	 * tab is the workspace.
	 */
	let { path, languages, source, reader, page }: { path: Crumb[]; languages: string[]; source: string; reader: string | null; page?: import('svelte').Snippet } = $props();

	const session = getCreatorSession();
	const doc = $derived(shell.doc);
	const saveWord = $derived(!doc ? null : doc.saving ? m['creator.saving']() : doc.dirty ? m['shell.unsaved']() : m['creator.saved']());

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
	<MainMenu {reader} />
	<a class="round" href="/" aria-current={shell.state.front === null ? 'page' : undefined} aria-label={m['shell.home']()} title={m['shell.home']()} data-testid="creator-brand"><Icon name="home" size="1rem" /></a>
	<TabStrip {path} />
	{#if page}<span class="page-tools">{@render page()}</span>{/if}
	<span class="spacer"></span>
	{#if doc}
		<button type="button" class="round" onclick={() => doc?.undo()} aria-label={m['history.undo']()} title={m['history.undo_hint']()} data-testid="bar-undo"><Icon name="rotate-ccw" size="1rem" /></button>
		<button type="button" class="round" onclick={() => doc?.redo()} aria-label={m['history.redo']()} title={m['history.redo_hint']()} data-testid="bar-redo"><Icon name="rotate-cw" size="1rem" /></button>
	{/if}
	{#if saveWord}
		<button type="button" class="pill state" class:dirty={doc?.dirty} disabled={doc?.saving} onclick={() => doc?.save()} title={m['creator.save_draft']()} data-testid="save-state">
			<span class="dot" aria-hidden="true"></span>{saveWord}
		</button>
	{/if}
	{#if languages.length > 1}
		<span class="pill lang"><TextLanguageSwitch {languages} {source} /></span>
	{/if}
	<span class="pill account" title={session.user?.email ?? ''}><Icon name="user" size="1rem" /><span class="who">{session.user?.email?.split('@')[0] ?? ''}</span></span>
	<span class="pill version" title={m['shell.version']()}>{version}</span>
</header>

<style>
	.shell-bar {
		display: flex;
		align-items: center;
		gap: 0.45rem;
		box-sizing: border-box;
		min-height: var(--shell-top, 3.6rem);
		padding: 0 1rem;
		background: var(--editor-stage);
		color: var(--editor-text);
		font-family: var(--editor-sans);
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
		background: var(--editor-surface);
		color: var(--editor-text);
		text-decoration: none;
	}
	.round {
		min-width: var(--control-height);
		border-radius: 50%;
	}
	.round:hover {
		background: var(--editor-hover);
	}
	.round[aria-current='page'] {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.pill {
		padding: 0 0.85rem;
		border-radius: 1.2rem;
		white-space: nowrap;
	}
	.state .dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--editor-go);
	}
	.state.dirty .dot {
		background: var(--editor-warn);
	}
	.lang {
		padding: 0 0.2rem;
	}
	.lang :global(.text-language.sc-control:not(:hover):not(:focus-within)) {
		border-color: transparent;
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
	.page-tools {
		display: flex;
		flex: none;
		align-items: center;
		gap: 0.25rem;
	}
	.spacer {
		flex: 1 1 0;
	}
</style>
