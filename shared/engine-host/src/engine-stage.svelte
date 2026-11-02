<script lang="ts">
	// <EngineStage> (#298): the being's canvas of an engine host. It makes the host, mounts it into its element and hands
	// it out (`bind:host`) – Start / Stop and the readouts are the page's (the host's `start()` needs a user gesture).
	// No app state, routing or paraglide in here.
	import { onMount, type Snippet } from 'svelte';
	import { createEngineHost } from './host.ts';
	import type { EngineHost } from './controller.ts';
	import type { EngineHostOptions } from './options.ts';

	let {
		options,
		host = $bindable(null),
		onmounted,
		children
	}: {
		/** read once, when the stage mounts */
		options: EngineHostOptions;
		host?: EngineHost | null;
		/** the deck and the being are there (the 3D may have failed – `host.space` is null then) */
		onmounted?: (host: EngineHost) => void;
		/** drawn over the canvas */
		children?: Snippet;
	} = $props();

	let element: HTMLDivElement | undefined = $state();

	onMount(() => {
		const h = createEngineHost(options);
		host = h;
		let alive = true;
		void (async () => {
			if (!element) return;
			await h.mount(element);
			if (alive) onmounted?.(h);
		})();
		return () => {
			alive = false;
			h.dispose();
			host = null;
		};
	});
</script>

<div class="engine-stage" bind:this={element} data-testid="engine-stage">
	{@render children?.()}
</div>

<style>
	.engine-stage {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
</style>
