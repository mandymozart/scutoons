<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { IconButton, Menu, Pill } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { showsTextLanguage } from '$lib/shell/crafts.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import TabStrip from '$lib/components/shell/tab-strip.svelte';
	import WorkspaceTabs from '$lib/components/shell/workspace-tabs.svelte';
	import MainMenu from '$lib/components/shell/main-menu.svelte';
	import UserMenu from '$lib/components/shell/user-menu.svelte';
	import TextLanguageSwitch from '$lib/components/text/text-language-switch.svelte';

	/**
	 * The bar (lap 2 A1; deck/editor/src/bar.rs): the open items' tabs at the left, the workspace row in the middle and
	 * nothing else there, round icons in pills at the right end – ↶ ↷, Publish (only while something in its scope is not
	 * live), the account (the user menu) and ☰ (the menu), far right. A craft may add its own: the translator's bar
	 * carries the text language (A9).
	 */
	let { reader = null }: { reader?: string | null } = $props();

	const doc = $derived(shell.doc);
	// a page that cannot tell (unpublished not given) keeps Publish; one that says nothing waits hides it
	const publishing = $derived(!!doc?.publish && doc.unpublished !== false);
	const languages = $derived(textLanguage.page.languages);
	const translating = $derived(showsTextLanguage(crafts.mine) && languages.length > 1);
</script>

<header class="shell-bar" data-testid="creator-bar" data-control-exempt="the bar's pills: deck-kit bar.rs's inset height">
	<TabStrip />
	<span class="middle"><WorkspaceTabs /></span>
	{#if translating}
		<Pill label={m['shell.text_language']()} data-testid="bar-text-language">
			<span class="globe" aria-hidden="true"><Icon name="globe" size="1rem" strokeWidth={2} /></span>
			<TextLanguageSwitch {languages} source={textLanguage.page.source} quiet />
		</Pill>
	{/if}
	{#if doc?.undo}
		<Pill label={m['history.undo']()}>
			<IconButton label={m['history.undo']()} title={m['history.undo_hint']()} icon="rotate-ccw" onclick={() => doc?.undo?.()} data-testid="bar-undo" />
			<IconButton label={m['history.redo']()} title={m['history.redo_hint']()} icon="rotate-cw" onclick={() => doc?.redo?.()} data-testid="bar-redo" />
		</Pill>
	{/if}
	<Pill>
		{#if publishing}
			<Menu label={m['shell.panel_publish']()} icon="upload-cloud" about={m['shell.publish_bar']()} testid="bar-publish" dot={!!doc?.unpublished} width="19rem">
				{#snippet children()}
					{#if doc?.publish}{@render doc.publish()}{/if}
				{/snippet}
			</Menu>
		{/if}
		<UserMenu />
		<MainMenu {reader} />
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
	.globe {
		display: inline-grid;
		place-items: center;
		padding-left: calc(var(--editor-pad) / 2);
	}
</style>
