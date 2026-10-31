<script lang="ts">
	// An XY pad of the browser deck (#197, #198): two parameters at once, x → right, y ↑ – the dot follows the values,
	// also when the being's 3D buttons turn them.
	import { PARAMS, toPos, toValue } from '@scutoons/live-deck';

	let {
		x,
		y,
		label,
		values,
		onset,
		ontouch
	}: {
		/** the parameters' paths (scene.ts PARAMS) */
		x: string;
		y: string;
		label: string;
		/** the deck's values now */
		values: Record<string, number>;
		onset: (path: string, value: number) => void;
		/** a finger on it (1) or off (0) – it lifts the deck's energy */
		ontouch: (on: 0 | 1) => void;
	} = $props();

	let pad: HTMLDivElement | undefined = $state();
	let on = $state(false);
	const left = $derived(toPos(PARAMS[x], values[x]) * 100);
	const top = $derived((1 - toPos(PARAMS[y], values[y])) * 100);
	const clamp = (t: number) => Math.min(1, Math.max(0, t));

	function move(e: PointerEvent) {
		if (!pad) return;
		const r = pad.getBoundingClientRect();
		onset(x, toValue(PARAMS[x], clamp((e.clientX - r.left) / r.width)));
		onset(y, toValue(PARAMS[y], clamp(1 - (e.clientY - r.top) / r.height)));
	}
	function down(e: PointerEvent) {
		pad?.setPointerCapture(e.pointerId);
		on = true;
		ontouch(1);
		move(e);
	}
	function up() {
		on = false;
		ontouch(0);
	}
</script>

<div
	class="pad"
	class:on
	bind:this={pad}
	role="slider"
	tabindex="-1"
	aria-label={label}
	aria-valuenow={values[x]}
	data-control-exempt="an XY pad: the whole square is the control"
	onpointerdown={down}
	onpointermove={(e) => pad?.hasPointerCapture(e.pointerId) && move(e)}
	onpointerup={up}
	onpointercancel={up}
>
	<i style:left="{left}%" style:top="{top}%"></i>
	<span>{label}</span>
</div>

<style>
	.pad {
		position: relative;
		aspect-ratio: 1;
		background: color-mix(in srgb, var(--ink) 70%, transparent);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 30%, transparent);
		touch-action: none;
		user-select: none;
	}
	.pad.on {
		border-color: var(--primary-cyan);
	}
	i {
		position: absolute;
		width: 2.5rem;
		height: 2.5rem;
		margin: -1.25rem 0 0 -1.25rem;
		border-radius: var(--radius-full);
		background: var(--primary-cyan);
		pointer-events: none;
	}
	span {
		position: absolute;
		left: var(--space-025);
		bottom: var(--space-025);
		right: var(--space-025);
		font-size: 0.75rem;
		color: color-mix(in srgb, var(--paper) 70%, transparent);
		pointer-events: none;
	}
</style>
