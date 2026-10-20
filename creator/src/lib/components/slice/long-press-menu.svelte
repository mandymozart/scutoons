<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Icon, clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The phone's long-press context (agents/research/TEXT-ACROSS-SLICES.md §4.1 E3, owner 2026-10-20): Copy · Cut ·
	 * Delete · Edit (the pencil) right at the element – four icon squares as the bottom row's, names as tooltip /
	 * aria-label, one frame of two regular border widths around the group. Above the element (below it when there is no
	 * room), inside the window; a tap elsewhere, a scroll or Esc closes it. Places in rem (DESIGN.md: sizes in rem, also
	 * from JS).
	 */
	let {
		at,
		oncopy,
		oncut,
		ondelete,
		onedit,
		onclose,
		testid = 'long-press-menu'
	}: {
		/** the element on screen */
		at: DOMRect;
		oncopy: () => void;
		oncut: () => void;
		ondelete: () => void;
		onedit: () => void;
		onclose: () => void;
		testid?: string;
	} = $props();

	let menu = $state<HTMLElement | null>(null);
	let place = $state<{ left: number; top: number } | null>(null);
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	onMount(() => {
		void tick().then(() => {
			const box = menu?.getBoundingClientRect();
			if (!box) return;
			const r = rem();
			const gap = 0.5 * r;
			const top = at.top - box.height - gap >= gap ? at.top - box.height - gap : Math.min(at.bottom + gap, innerHeight - box.height - gap);
			const left = Math.max(gap, Math.min(at.left, innerWidth - box.width - gap));
			place = { left: left / r, top: top / r };
			menu?.querySelector<HTMLButtonElement>('button')?.focus({ preventScroll: true });
		});
		const close = () => onclose();
		const key = (e: KeyboardEvent) => e.key === 'Escape' && onclose();
		window.addEventListener('scroll', close, { capture: true, passive: true });
		window.addEventListener('keydown', key);
		return () => {
			window.removeEventListener('scroll', close, { capture: true });
			window.removeEventListener('keydown', key);
		};
	});

	const run = (action: () => void) => () => {
		onclose();
		action();
	};
</script>

<div
	class="long-press"
	role="menu"
	tabindex="-1"
	bind:this={menu}
	style:left={place ? `${place.left}rem` : null}
	style:top={place ? `${place.top}rem` : null}
	style:visibility={place ? null : 'hidden'}
	{@attach clickOutside(onclose)}
	data-testid={testid}
>
	<button type="button" role="menuitem" title={m['creator.clip_copy']()} aria-label={m['creator.clip_copy']()} onclick={run(oncopy)} data-testid="{testid}-copy"><Icon name="copy" /></button>
	<button type="button" role="menuitem" title={m['creator.clip_cut_action']()} aria-label={m['creator.clip_cut_action']()} onclick={run(oncut)} data-testid="{testid}-cut"><Icon name="scissors" /></button>
	<button type="button" role="menuitem" title={m['creator.bar_delete']()} aria-label={m['creator.bar_delete']()} onclick={run(ondelete)} data-testid="{testid}-delete"><Icon name="trash-2" /></button>
	<button type="button" role="menuitem" title={m['creator.bar_edit']()} aria-label={m['creator.bar_edit']()} onclick={run(onedit)} data-testid="{testid}-edit"><Icon name="edit-2" /></button>
</div>

<style>
	.long-press {
		position: fixed;
		z-index: 40;
		display: flex;
		background: var(--paper);
		border: calc(2 * var(--border-width-regular)) solid var(--ink);
	}
	button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		min-width: 0;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--ink);
	}
	button:focus-visible {
		outline: var(--border-width-focus) solid var(--select-live-border);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
</style>
