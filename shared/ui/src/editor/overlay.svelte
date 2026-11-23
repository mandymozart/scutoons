<script module lang="ts">
	/** the overlays open now, the last on top: only the top one takes Esc */
	const open: symbol[] = [];
</script>

<script lang="ts">
	import { onMount, tick, type Snippet } from 'svelte';
	import { fade } from 'svelte/transition';
	import type { HTMLAttributes } from 'svelte/elements';
	import { motionMs } from './motion.ts';

	/**
	 * The overlay level (RULES H15; kit/tokens.toml `[component.overlay.*]`): over everything – the bar, the stage, the
	 * panels, the status bar – which it blurs and dims, so the place stays readable and nothing of it competes. The only
	 * blur of the tools; reduce motion (the OS's or the app's `html[data-motion='reduced']`) leaves a see-through dim.
	 * `place`: `top` (a column from the top that scrolls), `high` (a third down), `center`, `edge` (a box close to the
	 * window's edges), `veil` (nothing of its own: a native dialog above it, or what its owner raises over it). Esc and a
	 * click outside call `onclose`; without it the overlay stays until its content ends it.
	 */
	let {
		label,
		place = 'center',
		onclose,
		children,
		class: cls = '',
		...rest
	}: {
		label?: string;
		place?: 'top' | 'high' | 'center' | 'edge' | 'veil';
		onclose?: () => void;
		children?: Snippet;
		class?: string;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'class' | 'children'> = $props();

	const me = Symbol('overlay');
	// on the app's root, wherever its owner sits: a panel's or a page's stacking context must not hold it under the bar
	const lift = (node: HTMLElement) => {
		(node.ownerDocument.querySelector('.sc-tool') ?? node.ownerDocument.body).appendChild(node);
		return () => node.remove();
	};
	let root = $state<HTMLElement>();
	let box = $state<HTMLElement>();

	// the focus comes in (unless the content took it) and goes back where it was when the overlay closes
	onMount(() => {
		open.push(me);
		const was = document.activeElement as HTMLElement | null;
		if (place !== 'veil')
			void tick().then(() => {
				if (box && !box.contains(document.activeElement)) (box.querySelector<HTMLElement>('[autofocus]') ?? box).focus();
			});
		return () => {
			open.splice(open.indexOf(me), 1);
			if (place !== 'veil' && (!document.activeElement || document.activeElement === document.body || root?.contains(document.activeElement))) was?.focus?.();
		};
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'Escape' || event.defaultPrevented || !onclose || open.at(-1) !== me) return;
		// a native dialog above it takes its own Esc
		const dialog = event.target instanceof Element ? event.target.closest('dialog[open]') : null;
		if (dialog && !root?.contains(dialog)) return;
		event.preventDefault();
		onclose();
	}
	// outside: the veil, the layer, or – from the top – the column's own room beside what it carries
	function onclick(event: MouseEvent) {
		const t = event.target as Element | null;
		if (onclose && (t === root || t?.classList.contains('veil') || (place === 'top' && t === box))) onclose();
	}
</script>

<svelte:window {onkeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
	bind:this={root}
	class="overlay place-{place}"
	role={place === 'veil' ? undefined : 'dialog'}
	aria-modal={place === 'veil' ? undefined : 'true'}
	aria-label={label}
	data-overlay
	{onclick}
	{@attach lift}
	transition:fade={{ duration: motionMs('base') }}
>
	<div class="veil" aria-hidden="true"></div>
	{#if children && place !== 'veil'}
		<div bind:this={box} class="content {cls}" tabindex="-1" {...rest}>{@render children()}</div>
	{/if}
</div>

<style>
	.overlay {
		position: fixed;
		z-index: 90;
		inset: 0;
		display: flex;
		box-sizing: border-box;
		color: var(--tool-text);
		font-family: var(--editor-sans);
	}
	/* everything under it blurred and dimmed – the bar too (RULES H15) */
	.veil {
		position: absolute;
		inset: 0;
		background: var(--kit-overlay-rest-ground);
		backdrop-filter: blur(var(--kit-overlay-rest-blur));
	}
	/* reduce motion: no blur, a see-through dim */
	:global(html[data-motion='reduced']) .veil {
		background: var(--kit-overlay-reduced-ground);
		backdrop-filter: none;
	}
	@media (prefers-reduced-motion: reduce) {
		.veil {
			background: var(--kit-overlay-reduced-ground);
			backdrop-filter: none;
		}
	}
	.content {
		position: relative;
		box-sizing: border-box;
		min-width: 0;
		outline: none;
	}
	.place-top {
		overflow: auto;
	}
	.place-top .content {
		width: 100%;
		min-height: 100%;
	}
	.place-high {
		justify-content: center;
		align-items: flex-start;
		padding-top: 18vh;
	}
	.place-center {
		justify-content: center;
		align-items: center;
		padding: var(--kit-margin);
	}
	.place-high .content,
	.place-center .content {
		max-width: 100%;
		max-height: 100%;
	}
	/* a box close to the window's edges (a panel taken full): the kit's outer margin round it */
	.place-edge .content {
		position: absolute;
		inset: var(--kit-margin);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border: var(--editor-edge) solid var(--kit-overlay-box-border);
		border-radius: var(--kit-radius-box);
		background: var(--kit-overlay-box-ground);
		text-shadow: var(--tool-halo);
	}
	.place-edge .content :global(svg) {
		filter: var(--tool-icon-halo);
	}
</style>
