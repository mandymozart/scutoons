<script lang="ts">
	import { tick } from 'svelte';
	import { Icon, liveRegion } from '@scutoons/ui';
	import { clipboard } from '$lib/editor/clipboard.svelte.ts';
	import { isField } from '$lib/editor/editor-keys.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The clipboard's menu (BACKLOG 10.39 step 8 + phones): a right-click – on a phone the long-press, which the browser
	 * reports as the same `contextmenu` – inside a region that can copy (the slice canvas + inspector, the slice list,
	 * the chapter tree, a group's VN events) opens Copy · Cut · Paste · Paste settings at the pointer, acting on that
	 * (now live) region like Cmd/Ctrl+C / X / V / Shift+V. In a text field, on a sort grip or with page text selected the
	 * browser's own menu stays. `bar`: the same four as square buttons in one group – the ActionBar's "More" on phones,
	 * where no keys exist (DESIGN.md "Copy / paste").
	 */
	let { bar = false }: { bar?: boolean } = $props();

	let at = $state<{ x: number; y: number } | null>(null);
	let menu = $state<HTMLElement | null>(null);

	const textSelected = () => {
		const s = window.getSelection();
		return !!s && !s.isCollapsed && s.toString().trim() !== '';
	};

	async function onContextMenu(event: MouseEvent) {
		if (bar || event.defaultPrevented) return;
		const target = event.target as Element | null;
		if (!target || isField(target) || target.closest('[data-sort-grip], a') || textSelected()) return;
		const region = target.closest<HTMLElement>('[data-sc-region]')?.dataset.scRegion;
		if (!clipboard.handles(region)) return;
		event.preventDefault();
		if (region) liveRegion.set(region);
		at = { x: event.clientX, y: event.clientY };
		await tick();
		if (!menu) return;
		// inside the window: flip left / up where it would leave it
		const box = menu.getBoundingClientRect();
		at = { x: Math.max(0, Math.min(at.x, window.innerWidth - box.width)), y: Math.max(0, Math.min(at.y, window.innerHeight - box.height)) };
		menu.querySelector<HTMLButtonElement>('button')?.focus();
	}

	/** the pointer's place in rem (sizes and places in rem, also from JS – DESIGN.md) */
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	function close() {
		at = null;
	}

	async function run(action: 'copy' | 'cut' | 'paste' | 'settings') {
		close();
		if (action === 'copy' || action === 'cut') await clipboard.copyFromMenu(action === 'cut');
		else await clipboard.pasteFromMenu(action === 'settings');
	}

	function onKeydown(event: KeyboardEvent) {
		if (!at) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			close();
			return;
		}
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		const items = [...(menu?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
		const i = items.indexOf(document.activeElement as HTMLButtonElement);
		items[(i + (event.key === 'ArrowDown' ? 1 : items.length - 1)) % items.length]?.focus();
		event.preventDefault();
	}

	function onPointerDown(event: PointerEvent) {
		if (at && !menu?.contains(event.target as Node)) close();
	}
</script>

<svelte:window oncontextmenu={onContextMenu} onkeydown={onKeydown} onpointerdown={onPointerDown} onscroll={close} onresize={close} />

{#snippet items()}
	<button type="button" onclick={() => run('copy')} data-testid="clip-copy"><Icon name="copy" /> {m['creator.clip_copy']()}</button>
	<button type="button" onclick={() => run('cut')} data-testid="clip-cut"><Icon name="scissors" /> {m['creator.clip_cut_action']()}</button>
	<button type="button" onclick={() => run('paste')} data-testid="clip-paste"><Icon name="clipboard" /> {m['creator.clip_paste']()}</button>
	<button type="button" onclick={() => run('settings')} data-testid="clip-paste-settings">{m['creator.clip_paste_settings']()}</button>
{/snippet}

{#if bar}
	<span class="clip-bar sc-seg" role="group" aria-label={m['creator.clip_menu']()} data-testid="clip-bar">{@render items()}</span>
{:else if at}
	<div class="clip-menu" role="menu" aria-label={m['creator.clip_menu']()} bind:this={menu} style:left="{at.x / rem()}rem" style:top="{at.y / rem()}rem" data-testid="clip-menu">
		{@render items()}
	</div>
{/if}

<style>
	.clip-menu {
		position: fixed;
		z-index: 200;
		display: flex;
		flex-direction: column;
		min-width: 12rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
	}
	.clip-menu button {
		display: flex;
		align-items: center;
		/* a menu reads from the left (tools.css centres a button's content) */
		justify-content: flex-start;
		gap: var(--space-05);
		min-height: var(--control-height);
		padding: 0 var(--space-1);
		border: 0;
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.clip-menu button:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.clip-menu button:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.clip-bar {
		display: inline-flex;
		flex-wrap: wrap;
	}
</style>
