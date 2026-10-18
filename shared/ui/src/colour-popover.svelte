<script lang="ts">
	import { HEX, brandPalette, hexToHsv, hsvToHex, longHex, recentColours, type Hsv } from './colour.ts';
	import { uiLabels } from './ui-labels.ts';
	import { untrack } from 'svelte';

	/**
	 * The colour popover's content (ColorField, owner 2026-10-18: "the native picker isn't app UI style"): the brand palette
	 * from the tokens (plus a `palette` the app passes, e.g. a toon's), this browser's recent colours, a saturation /
	 * brightness area with a hue slider, and the hex. Every pick goes out at once (`onpick`, a whole `#rrggbb`); the field
	 * stays the source of the value. Keyboard: Tab through swatches → area (arrows: saturation ← →, brightness ↑ ↓; Shift
	 * ×10) → hue (a range input) → hex; touch: tap a swatch, drag in the area. Sizes in rem, ink frames, square.
	 */
	let {
		value,
		fallback,
		palette = [],
		onpick
	}: {
		/** the field's colour (a whole hex) or none */
		value: string | null | undefined;
		/** the colour shown when unset (the field's placeholder colour) */
		fallback?: string | null;
		/** extra swatches first (name + hex), e.g. a toon's palette */
		palette?: { name: string; hex: string }[];
		onpick: (hex: string) => void;
	} = $props();

	const t = uiLabels();
	const swatches = $derived([...palette.filter((c) => HEX.test(c.hex)).map((c) => ({ ...c, hex: longHex(c.hex) })), ...brandPalette()]);
	const recent = recentColours();
	const current = $derived(typeof value === 'string' && HEX.test(value) ? longHex(value) : null);

	/** the picker's own state (keeps the hue while saturation or brightness is 0) */
	let hsv = $state<Hsv>(untrack(() => hexToHsv(HEX.test(value ?? '') ? value! : HEX.test(fallback ?? '') ? fallback! : '#000000')));
	// a colour set from outside (a swatch, the hex, the field) moves the picker – not the echo of its own pick
	let last = '';
	$effect(() => {
		if (current && current !== last) hsv = hexToHsv(current);
	});

	function pick(hex: string) {
		last = longHex(hex);
		onpick(last);
	}
	/** a whole colour chosen (a swatch, the hex): the picker moves there too */
	function choose(hex: string) {
		hsv = hexToHsv(hex);
		pick(hex);
	}
	function set(next: Partial<Hsv>) {
		hsv = { ...hsv, ...next };
		pick(hsvToHex(hsv));
	}

	// ── the saturation / brightness area ──
	let area: HTMLDivElement | undefined = $state();
	let dragging = false;
	function fromPointer(e: PointerEvent) {
		const r = area!.getBoundingClientRect();
		set({ s: Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)), v: Math.min(1, Math.max(0, 1 - (e.clientY - r.top) / r.height)) });
	}
	function areaDown(e: PointerEvent) {
		if (e.button !== 0) return;
		e.preventDefault();
		area?.focus();
		try {
			area?.setPointerCapture(e.pointerId);
		} catch {
			/* synthetic */
		}
		dragging = true;
		fromPointer(e);
	}
	function areaKey(e: KeyboardEvent) {
		const d = e.shiftKey ? 0.1 : 0.01;
		const moves: Record<string, Partial<Hsv>> = {
			ArrowLeft: { s: Math.max(0, hsv.s - d) },
			ArrowRight: { s: Math.min(1, hsv.s + d) },
			ArrowDown: { v: Math.max(0, hsv.v - d) },
			ArrowUp: { v: Math.min(1, hsv.v + d) }
		};
		if (!(e.key in moves)) return;
		e.preventDefault();
		set(moves[e.key]);
	}

	/** the hex field: only a whole colour goes out while typing (as in ColorField) */
	function typeHex(raw: string) {
		const v = raw.trim();
		const hex = v.startsWith('#') ? v : `#${v}`;
		if (HEX.test(hex)) choose(hex);
	}
</script>

<div class="sc-colour-popover">
	{#snippet row(list: { name: string; hex: string }[], heading: string, id: string)}
		<div class="section" role="group" aria-labelledby={id}>
			<span class="sc-label" {id}>{heading}</span>
			<div class="swatches">
				{#each list as c (c.name + c.hex)}
					<button
						type="button"
						class="swatch"
						style:--c={c.hex}
						aria-label={c.name ? `${c.name} ${c.hex}` : c.hex}
						title={c.name ? `${c.name} ${c.hex}` : c.hex}
						aria-pressed={current === c.hex}
						onclick={() => choose(c.hex)}
					></button>
				{/each}
			</div>
		</div>
	{/snippet}
	{@render row(swatches, t('palette'), 'cp-palette')}
	{#if recent.length}{@render row(recent.map((hex) => ({ name: '', hex })), t('recent'), 'cp-recent')}{/if}

	<div
		bind:this={area}
		class="area"
		role="slider"
		tabindex="0"
		aria-label={t('shade')}
		aria-valuemin={0}
		aria-valuemax={100}
		aria-valuenow={Math.round(hsv.s * 100)}
		aria-valuetext="{Math.round(hsv.s * 100)} % · {Math.round(hsv.v * 100)} %"
		style:--hue={hsv.h}
		onpointerdown={areaDown}
		onpointermove={(e) => dragging && fromPointer(e)}
		onpointerup={() => (dragging = false)}
		onpointercancel={() => (dragging = false)}
		onkeydown={areaKey}
	>
		<span class="dot" style:left="{hsv.s * 100}%" style:top="{(1 - hsv.v) * 100}%"></span>
	</div>
	<input class="hue" type="range" min="0" max="359" step="1" aria-label={t('hue')} value={Math.round(hsv.h)} oninput={(e) => set({ h: Number(e.currentTarget.value) })} />
	<input class="hex" type="text" spellcheck="false" autocomplete="off" aria-label={t('hex')} value={current ?? ''} placeholder={fallback ?? ''} oninput={(e) => typeHex(e.currentTarget.value)} />
</div>

<style>
	.sc-colour-popover {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		box-sizing: border-box;
		width: 16rem;
		padding: var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	.section {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.swatches {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025);
	}
	/* a swatch: a control-height square, an ink frame, a paper ring (padding), the colour inside; the current one a
	   thicker frame (no shadows – DESIGN.md) */
	.swatch {
		box-sizing: border-box;
		width: var(--control-height);
		margin: 0;
		padding: var(--border-width-regular);
		border: var(--border-width-regular) solid var(--ink);
		background: linear-gradient(var(--c), var(--c)) content-box, var(--paper);
		cursor: pointer;
	}
	.swatch[aria-pressed='true'] {
		border-width: calc(2 * var(--border-width-regular));
	}
	.area {
		position: relative;
		height: 8rem;
		border: var(--border-width-regular) solid var(--ink);
		background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(var(--hue) 100% 50%));
		cursor: crosshair;
		touch-action: none;
	}
	.dot {
		position: absolute;
		width: 0.75rem;
		height: 0.75rem;
		transform: translate(-50%, -50%);
		border: var(--border-width-regular) solid var(--paper);
		outline: var(--border-width-hairline) solid var(--ink);
		pointer-events: none;
	}
	.hue {
		appearance: none;
		width: 100%;
		height: 1rem;
		margin: 0;
		border: var(--border-width-regular) solid var(--ink);
		background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00);
		cursor: pointer;
	}
	.hue::-webkit-slider-thumb {
		appearance: none;
		width: 0.5rem;
		height: 1.25rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.hue::-moz-range-thumb {
		box-sizing: border-box;
		width: 0.5rem;
		height: 1.25rem;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 0;
		background: var(--paper);
	}
	.hex {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05);
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	.hex:focus {
		border-color: var(--ink);
	}
</style>
