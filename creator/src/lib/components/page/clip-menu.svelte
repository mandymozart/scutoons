<script lang="ts">
	import { tick } from 'svelte';
	import { liveRegion } from '@scutoons/ui';
	import { MenuItem, MenuPanel } from '@scutoons/ui/editor';
	import { clipboard } from '$lib/editor/clipboard.svelte.ts';
	import { isField } from '$lib/editor/editor-keys.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The clipboard's menu: a right-click inside a region that can copy (the slice canvas + inspector, the slice list, the
	 * chapter tree, a group's VN events) opens Copy · Cut · Paste · Paste settings at the pointer, acting on that (now live)
	 * region like Cmd/Ctrl+C / X / V / Shift+V. In a text field, on a sort grip or with page text selected the browser's own
	 * menu stays (DESIGN.md "Copy / paste").
	 */
	let at = $state<{ x: number; y: number } | null>(null);
	let menu = $state<HTMLElement | null>(null);

	const textSelected = () => {
		const s = window.getSelection();
		return !!s && !s.isCollapsed && s.toString().trim() !== '';
	};

	async function onContextMenu(event: MouseEvent) {
		if (event.defaultPrevented) return;
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

{#if at}
	<div class="clip-menu" bind:this={menu} style:left="{at.x / rem()}rem" style:top="{at.y / rem()}rem">
		<MenuPanel width="12rem" aria-label={m['creator.clip_menu']()} data-testid="clip-menu">
			<MenuItem icon="copy" label={m['creator.clip_copy']()} onclick={() => run('copy')} data-testid="clip-copy" />
			<MenuItem icon="scissors" label={m['creator.clip_cut_action']()} onclick={() => run('cut')} data-testid="clip-cut" />
			<MenuItem icon="clipboard" label={m['creator.clip_paste']()} onclick={() => run('paste')} data-testid="clip-paste" />
			<MenuItem label={m['creator.clip_paste_settings']()} onclick={() => run('settings')} data-testid="clip-paste-settings" />
		</MenuPanel>
	</div>
{/if}

<style>
	.clip-menu {
		position: fixed;
		z-index: 200;
	}
</style>
