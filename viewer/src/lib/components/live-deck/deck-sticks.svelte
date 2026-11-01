<script lang="ts">
	// The browser deck's two sticks (#209, owner 2026-10-31: "the two-axis controls become mobile-game sticks: two
	// overlays in the left and right corners … invisible when not dragged … a touch in the corner area shows the stick
	// where the thumb lands, and it fades out on release … these areas don't move the scene"). Two invisible corner
	// zones over the 3D: a touch (or the mouse) that starts in one is the stick's – the canvas never sees it, so it
	// doesn't orbit or zoom. What a stick drives is the selection's (deck-space controls, via `paths`); the deflection
	// goes out in stick radii, y up.
	// #225 (owner 2026-10-31: "the two thumb regions are the whole bottom half and take away room to play … they jump to
	// wherever you touch … much less space, rather down in the corner"): small corner zones, each stick at a fixed
	// place in its corner (a faint ring at rest), the deflection from that ring's centre. The panel switches them off.
	import { deflection } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		labels,
		ondown,
		onmove,
		onup
	}: {
		/** the [x, y] labels stick `side` drives now ("" = none) */
		labels: (side: number) => string[];
		ondown: (side: number) => void;
		/** the deflection, −1..1 each, y up */
		onmove: (side: number, dx: number, dy: number) => void;
		onup: (side: number) => void;
	} = $props();

	interface Thumb {
		id: number;
		at: [number, number];
		now: [number, number];
		live: boolean;
	}
	let thumbs = $state<(Thumb | null)[]>([null, null]);
	let shown = $state<string[][]>([[], []]);
	/** CSS px per rem (positions go out in rem – RULES "Sizes in rem") and the stick's reach: 2.5rem */
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize || '16') || 16;
	const REACH = 2.5;
	const radius = () => REACH * rem();

	function local(e: PointerEvent): [number, number] {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		return [e.clientX - r.left, e.clientY - r.top];
	}
	/** the stick's fixed place: its zone's centre */
	function centre(e: PointerEvent): [number, number] {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		return [r.width / 2, r.height / 2];
	}
	function down(side: number, e: PointerEvent) {
		if (thumbs[side]?.live) return;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		thumbs[side] = { id: e.pointerId, at: centre(e), now: local(e), live: true };
		shown[side] = labels(side);
		ondown(side);
		e.preventDefault();
	}
	function move(side: number, e: PointerEvent) {
		const t = thumbs[side];
		if (!t?.live || t.id !== e.pointerId) return;
		t.now = local(e);
		const [dx, dy] = deflection(t.at, t.now, radius());
		onmove(side, dx, dy);
	}
	function up(side: number, e: PointerEvent) {
		const t = thumbs[side];
		if (!t?.live || t.id !== e.pointerId) return;
		t.live = false;
		t.now = t.at;
		onup(side);
	}
	/** the thumb's knob inside the ring (rem from the landing point) */
	function knob(t: Thumb): [number, number] {
		const [dx, dy] = deflection(t.at, t.now, radius());
		return [dx * REACH, -dy * REACH];
	}
	/** the knob's CSS transform – centred at rest */
	function place(t: Thumb | null): string {
		const [x, y] = t ? knob(t) : [0, 0];
		return `translate(${x}rem, ${y}rem)`;
	}
</script>

{#each [0, 1] as side (side)}
	{@const t = thumbs[side]}
	<div
		class="zone"
		class:right={side === 1}
		role="slider"
		tabindex="-1"
		aria-label={side === 0 ? m['live_deck.stick_left']() : m['live_deck.stick_right']()}
		aria-valuenow={0}
		data-testid="deck-stick-{side}"
		data-control-exempt="a game stick: the whole corner is the control"
		onpointerdown={(e) => down(side, e)}
		onpointermove={(e) => move(side, e)}
		onpointerup={(e) => up(side, e)}
		onpointercancel={(e) => up(side, e)}
		oncontextmenu={(e) => e.preventDefault()}
	>
		<div class="stick" class:live={t?.live}>
			<i style:transform={place(t)}></i>
			{#if t?.live}
				<span class="x">{shown[side][0] ?? ''}</span>
				<span class="y">{shown[side][1] ?? ''}</span>
			{/if}
		</div>
	</div>
{/each}

<style>
	/* the corners: small thumb-sized squares above the panel's bar (#225), the stick fixed in the middle of each */
	.zone {
		position: absolute;
		left: 0.5rem;
		bottom: calc(var(--control-height) + 0.75rem + env(safe-area-inset-bottom));
		width: 7.5rem;
		height: 7.5rem;
		touch-action: none;
		user-select: none;
		-webkit-touch-callout: none;
		outline: none;
	}
	.zone.right {
		left: auto;
		right: 0.5rem;
	}
	/* at rest a faint ring, so the thumb knows where it is; bright while held */
	.stick {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 5rem;
		height: 5rem;
		margin: -2.5rem 0 0 -2.5rem;
		border-radius: var(--radius-full);
		border: var(--border-width-regular) solid color-mix(in srgb, var(--paper) 45%, transparent);
		background: color-mix(in srgb, var(--ink) 25%, transparent);
		opacity: 0.3;
		transition: opacity 0.45s ease-out;
		pointer-events: none;
	}
	.stick.live {
		opacity: 1;
		transition: opacity 0.08s;
	}
	.stick.live i {
		transition: none;
	}
	i {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 2rem;
		height: 2rem;
		margin: -1rem 0 0 -1rem;
		transition: transform 0.15s ease-out;
		border-radius: var(--radius-full);
		background: color-mix(in srgb, var(--primary-cyan) 85%, transparent);
	}
	span {
		position: absolute;
		font-family: var(--font-mono);
		font-size: 0.5625rem;
		text-transform: uppercase;
		white-space: nowrap;
		color: var(--paper);
		text-shadow: 0 0 0.25rem var(--ink);
	}
	/* x along the bottom, y up the side */
	.x {
		left: 50%;
		top: calc(100% + 0.25rem);
		transform: translateX(-50%);
	}
	.y {
		left: calc(100% + 0.25rem);
		top: 50%;
		transform: translateY(-50%);
	}
	.zone.right .y {
		left: auto;
		right: calc(100% + 0.25rem);
	}
	@media (prefers-reduced-motion: reduce) {
		.stick,
		i {
			transition: none;
		}
	}
</style>
