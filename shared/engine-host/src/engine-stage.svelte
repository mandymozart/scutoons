<script lang="ts">
	// <EngineStage> (#298): the being's canvas of an engine host. It makes the host, mounts it into its element and hands
	// it out (`bind:host`) – Start / Stop and the readouts are the page's (the host's `start()` needs a user gesture).
	// `gate` (opt-in, owner 2026-11-02): a round Play button centred over the canvas, and the first user gesture anywhere on
	// the element starts the sound too (gate.ts) – the browser blocks the first sound without one. No app state, routing or
	// paraglide in here.
	import { onMount, type Snippet } from 'svelte';
	import { createEngineHost } from './host.ts';
	import type { EngineHost } from './controller.ts';
	import type { EngineHostOptions } from './options.ts';
	import { StartGate, gateOpen, listenForGesture, type GateState } from './gate.ts';

	let {
		options,
		host = $bindable(null),
		gate = false,
		onmounted,
		children
	}: {
		/** read once, when the stage mounts */
		options: EngineHostOptions;
		host?: EngineHost | null;
		/** a Play button over the canvas and the first gesture on it start the sound (default off: the page has its own Start) */
		gate?: boolean;
		/** the deck and the being are there (the 3D may have failed – `host.space` is null then) */
		onmounted?: (host: EngineHost) => void;
		/** drawn over the canvas */
		children?: Snippet;
	} = $props();

	let element: HTMLDivElement | undefined = $state();
	let gateState = $state<GateState>('idle');
	let starter: StartGate | null = null;

	onMount(() => {
		const h = createEngineHost(options);
		host = h;
		const off: (() => void)[] = [];
		if (gate && element) {
			const g = new StartGate(() => h.start(), (s) => (gateState = s));
			starter = g;
			off.push(listenForGesture(element, g));
			// the page's own Start button (or anything else) started the sound: the gate is done
			off.push(h.subscribe((s) => s.phase === 'running' && g.markRunning()));
		}
		let alive = true;
		void (async () => {
			if (!element) return;
			await h.mount(element);
			if (alive) onmounted?.(h);
		})();
		return () => {
			alive = false;
			for (const f of off) f();
			starter = null;
			h.dispose();
			host = null;
		};
	});
</script>

<div class="engine-stage" bind:this={element} data-testid="engine-stage" style="position: relative; width: 100%; height: 100%; min-height: 0; overflow: hidden;">
	{@render children?.()}
	{#if gate && gateOpen(gateState)}
		<div class="engine-gate" style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.75rem; pointer-events: none;">
			<button type="button" class="sc-round" title="Play" aria-label="Play" data-testid="engine-play" style="pointer-events: auto;" onclick={() => void starter?.gesture()}>
				<svg viewBox="0 0 24 24" aria-hidden="true" style="width: 1.5rem; height: 1.5rem;"><path d="M8 5v14l11-7z" fill="currentColor" /></svg>
			</button>
			{#if gateState === 'blocked'}
				<p data-testid="engine-gate-note" style="margin: 0; color: var(--text-muted);">The browser held the sound back – press Play again.</p>
			{/if}
		</div>
	{/if}
</div>
