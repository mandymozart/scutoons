<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	/**
	 * The kit's range slider (kit/tokens.toml `component.slider`): a thin within track, the filled part and a round knob in
	 * the text colour, no hue. A native range underneath, so the keys and assistive technology are the platform's; `label`
	 * names it; `--slider-length` on a parent sets its length. `vertical` stands it up (a fader, the top the most);
	 * `fill={false}` leaves the track to the caller's `--slider-ground` and `--slider-thickness` (the colour pick's hue).
	 */
	type Props = {
		value: number;
		label: string;
		min?: number;
		max?: number;
		step?: number | 'any';
		vertical?: boolean;
		fill?: boolean;
		oninput?: (value: number) => void;
		onchange?: (value: number) => void;
		'data-kit-state'?: string;
	} & Omit<HTMLInputAttributes, 'type' | 'value' | 'min' | 'max' | 'step' | 'oninput' | 'onchange'>;

	let { value = $bindable(), label, min = 0, max = 100, step = 1, vertical = false, fill = true, oninput, onchange, title, ...rest }: Props = $props();
</script>

<input
	{...rest}
	type="range"
	class="kit-slider"
	class:vertical
	class:fill
	{min}
	{max}
	{step}
	bind:value
	aria-label={label}
	title={title ?? label}
	aria-orientation={vertical ? 'vertical' : undefined}
	oninput={(e) => oninput?.(Number(e.currentTarget.value))}
	onchange={(e) => onchange?.(Number(e.currentTarget.value))}
/>

<style>
	.kit-slider {
		--slider-track: var(--slider-thickness, 0.25rem);
		--slider-knob: 0.875rem;
		appearance: none;
		box-sizing: border-box;
		width: var(--slider-length, auto);
		height: var(--slider-knob);
		margin: 0;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
	}
	.vertical {
		writing-mode: vertical-lr;
		direction: rtl;
		width: var(--slider-knob);
		height: var(--slider-length, 8rem);
	}
	.kit-slider::-webkit-slider-runnable-track {
		height: var(--slider-track);
		border-radius: calc(var(--slider-track) / 2);
		background: var(--slider-ground, var(--kit-slider-rest-ground));
	}
	.kit-slider::-moz-range-track {
		height: var(--slider-track);
		border-radius: calc(var(--slider-track) / 2);
		background: var(--slider-ground, var(--kit-slider-rest-ground));
	}
	.vertical::-webkit-slider-runnable-track {
		width: var(--slider-track);
		height: auto;
	}
	.vertical::-moz-range-track {
		width: var(--slider-track);
		height: auto;
	}
	.fill::-moz-range-progress {
		height: var(--slider-track);
		border-radius: calc(var(--slider-track) / 2);
		background: var(--kit-slider-rest-mark);
	}
	.vertical.fill::-moz-range-progress {
		width: var(--slider-track);
		height: auto;
	}
	.kit-slider::-webkit-slider-thumb {
		appearance: none;
		width: var(--slider-knob);
		height: var(--slider-knob);
		margin-top: calc((var(--slider-track) - var(--slider-knob)) / 2);
		border: 0;
		border-radius: 50%;
		background: var(--kit-slider-rest-knob);
	}
	.vertical::-webkit-slider-thumb {
		margin-top: 0;
		margin-left: calc((var(--slider-track) - var(--slider-knob)) / 2);
	}
	.kit-slider::-moz-range-thumb {
		width: var(--slider-knob);
		height: var(--slider-knob);
		border: 0;
		border-radius: 50%;
		background: var(--kit-slider-rest-knob);
	}
	.kit-slider[data-kit-state~='hover']::-webkit-slider-thumb {
		background: var(--kit-slider-hover-knob);
	}
	.kit-slider[data-kit-state~='hover']::-moz-range-thumb {
		background: var(--kit-slider-hover-knob);
	}
	@media (hover: hover) {
		.kit-slider:hover:not(:disabled)::-webkit-slider-thumb {
			background: var(--kit-slider-hover-knob);
		}
		.kit-slider:hover:not(:disabled)::-moz-range-thumb {
			background: var(--kit-slider-hover-knob);
		}
	}
	.kit-slider:focus-visible,
	.kit-slider[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.kit-slider:disabled {
		opacity: var(--kit-slider-disabled-opacity);
		cursor: not-allowed;
	}
</style>
