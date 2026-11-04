<script lang="ts">
	// A rotary knob of the browser deck (#209, owner 2026-10-31: "sliders become circular knobs, like Ableton's (rotary,
	// drag up / down)"): a 270° arc lit to the value, drag up / down to turn it (Shift: fine), a double click back to the
	// scene's value, the arrow keys step it. A tap (no turn) picks it – it owns the sticks, and in MIDI learn it waits for
	// a controller. The mapped control shows under it. Where the value sits, a drag's and a key's turn: the core's (#369,
	// DeckControls); here the pointer, the keys and the arc.
	import { arcPoint, type KnobTurns, type Spec } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		path,
		spec,
		value,
		turns,
		mapped = '',
		selected = false,
		learning = false,
		onset,
		onpick,
		ontouch
	}: {
		path: string;
		spec: Spec;
		value: number;
		/** the core's controls: the value's position, a drag, a key (null: none yet – the knob shows its start) */
		turns: KnobTurns | null;
		/** what drives it over MIDI ("CC 7 · 1") */
		mapped?: string;
		/** it owns the sticks */
		selected?: boolean;
		/** MIDI learn waits for a controller for it */
		learning?: boolean;
		onset: (path: string, value: number) => void;
		/** a tap: select it (and learn, in MIDI learn) */
		onpick: (path: string) => void;
		ontouch: (on: 0 | 1) => void;
	} = $props();

	const R = 9;
	const C = 12;
	const t = $derived(turns ? turns.norm(path, value) : 0);
	const arc = (to: number) => {
		const [x0, y0] = arcPoint(0, R, C);
		const [x1, y1] = arcPoint(to, R, C);
		return `M ${x0.toFixed(3)} ${y0.toFixed(3)} A ${R} ${R} 0 ${to * 270 > 180 ? 1 : 0} 1 ${x1.toFixed(3)} ${y1.toFixed(3)}`;
	};
	const tip = $derived(arcPoint(t, R - 3.5, C));
	const shown = $derived(value.toFixed(Math.abs(value) >= 100 ? 0 : Math.abs(value) >= 10 ? 1 : 2));

	let drag: { id: number; y: number; start: number; moved: boolean } | null = null;
	/** a full turn over 8rem of drag (rem, not px – RULES "Sizes in rem") */
	const range = () => 8 * parseFloat(getComputedStyle(document.documentElement).fontSize || '16');

	function down(e: PointerEvent) {
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		drag = { id: e.pointerId, y: e.clientY, start: value, moved: false };
		ontouch(1);
		e.preventDefault();
	}
	function move(e: PointerEvent) {
		if (!drag || drag.id !== e.pointerId) return;
		const dy = e.clientY - drag.y;
		if (!drag.moved && Math.abs(dy) < 3) return;
		drag.moved = true;
		const v = turns?.turn(path, drag.start, dy, range(), e.shiftKey);
		if (v !== undefined) onset(path, v);
	}
	function up(e: PointerEvent) {
		if (!drag || drag.id !== e.pointerId) return;
		if (!drag.moved) onpick(path);
		drag = null;
		ontouch(0);
	}
	function key(e: KeyboardEvent) {
		const v = turns?.step(path, value, e.key, e.shiftKey);
		if (v === undefined) return;
		onset(path, v);
		e.preventDefault();
	}
</script>

<div
	class="knob"
	class:selected
	class:learning
	role="slider"
	tabindex="0"
	aria-label={spec.label}
	aria-valuemin={spec.min}
	aria-valuemax={spec.max}
	aria-valuenow={value}
	title={m['live_deck.knob_hint']({ label: spec.label })}
	data-testid="deck-knob-{path}"
	data-control-exempt="a rotary knob: the dial is the control"
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
	ondblclick={() => onset(path, spec.value)}
	onkeydown={key}
>
	<svg viewBox="0 0 24 24" aria-hidden="true">
		<path class="track" d={arc(1)} />
		{#if t > 0.002}<path class="value" d={arc(t)} />{/if}
		<line x1={C} y1={C} x2={tip[0]} y2={tip[1]} />
	</svg>
	<span class="label">{spec.label}</span>
	<span class="num">{learning ? 'MIDI…' : mapped || shown}</span>
</div>

<style>
	.knob {
		display: grid;
		justify-items: center;
		gap: 0.0625rem;
		min-width: 0;
		padding: var(--space-025) 0;
		cursor: ns-resize;
		touch-action: none;
		user-select: none;
		font-family: var(--font-mono);
		line-height: 1;
		outline: none;
	}
	svg {
		width: 2.25rem;
		height: 2.25rem;
		overflow: visible;
	}
	path {
		fill: none;
		stroke-width: 2.5;
		stroke-linecap: butt;
	}
	.track {
		stroke: color-mix(in srgb, var(--paper) 18%, transparent);
	}
	.value {
		stroke: var(--primary-cyan);
	}
	line {
		stroke: var(--paper);
		stroke-width: 1.5;
		stroke-linecap: round;
	}
	.label,
	.num {
		max-width: 100%;
		font-size: 0.5625rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-transform: uppercase;
		letter-spacing: 0.02em;
	}
	.label {
		color: color-mix(in srgb, var(--paper) 75%, transparent);
	}
	.num {
		color: color-mix(in srgb, var(--paper) 50%, transparent);
		font-variant-numeric: tabular-nums;
	}
	.knob:focus-visible svg,
	.selected svg {
		filter: drop-shadow(0 0 0.25rem var(--primary-cyan));
	}
	.selected .label {
		color: var(--paper);
	}
	.learning .num {
		color: var(--state-warning);
	}
	.learning svg {
		animation: blink 0.6s steps(2, jump-none) infinite;
	}
	@keyframes blink {
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.learning svg {
			animation: none;
		}
	}
</style>
