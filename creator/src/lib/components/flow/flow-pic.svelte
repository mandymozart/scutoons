<script lang="ts" module>
	import { getContext, setContext } from 'svelte';

	/** the graph's lazy loading: one IntersectionObserver on the canvas – a picture loads when its box comes near the view */
	export type Watch = (el: Element, near: () => void) => () => void;
	const KEY = Symbol('flow-lazy');
	export function lazyPictures(root: () => Element | null): Watch {
		let observer: IntersectionObserver | null = null;
		const waiting = new Map<Element, () => void>();
		const watch: Watch = (el, near) => {
			if (typeof IntersectionObserver === 'undefined') return (near(), () => {});
			// a box within one canvas size of the view (rootMargin 100%) loads; the rest waits until the author pans there
			observer ??= new IntersectionObserver(
				(entries) => {
					for (const e of entries) {
						if (!e.isIntersecting) continue;
						waiting.get(e.target)?.();
						waiting.delete(e.target);
						observer?.unobserve(e.target);
					}
				},
				{ root: root(), rootMargin: '100%' }
			);
			waiting.set(el, near);
			observer.observe(el);
			return () => (waiting.delete(el), observer?.unobserve(el));
		};
		setContext(KEY, watch);
		return watch;
	}
</script>

<script lang="ts">
	/**
	 * A small picture of a Flow box (owner 2026-10-06): loaded only when its box comes near the view; nothing to show
	 * (no picture, not found, a broken file) = the dashed empty slot, never a broken image. `title` = its name.
	 */
	let { src, title, shape = 'square', testid }: { src?: string; title?: string; shape?: 'square' | 'round'; testid?: string } = $props();

	const watch = getContext<Watch | undefined>(KEY);
	let el = $state<HTMLElement | null>(null);
	let near = $state(!watch);
	let failed = $state(false);
	$effect(() => {
		if (!el || near || !watch) return;
		return watch(el, () => (near = true));
	});
	$effect(() => {
		void src;
		failed = false;
	});
	const empty = $derived(!src || failed);
</script>

<span class="pic {shape}" class:empty bind:this={el} {title} data-testid={testid} data-src={empty ? undefined : src}>
	{#if near && !empty}
		<img {src} alt={title ?? ''} draggable="false" decoding="async" onerror={() => (failed = true)} />
	{/if}
</span>

<style>
	.pic {
		display: block;
		flex: none;
		box-sizing: border-box;
		overflow: hidden;
		background: var(--paper);
		border: var(--border-width-hairline) solid var(--ink);
	}
	.round {
		border-radius: 50%;
	}
	/* nothing to show: the dashed empty slot (DESIGN.md: an empty media slot is dashed) */
	.empty {
		border-style: dashed;
		border-color: var(--border-muted);
	}
	img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: top;
	}
</style>
