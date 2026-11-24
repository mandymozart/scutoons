<script lang="ts">
	import { isSwitchKey, switchKeyDown } from './toggle-keys.ts';

	/**
	 * The editor's on/off switch: a track with a knob that slides right and fills the track while on – a
	 * `<button role="switch">`, Space and Enter flip it once per press. Every boolean of a panel is this switch, never a
	 * checkbox or an on/off segmented control. The row's word is its visible label; `label` names it for assistive
	 * technology; `words` adds a quiet value after the track where the value has words of its own; `shown` writes the label
	 * after the track, inside the switch, where no row word stands beside it (a form's ticked line).
	 */
	let {
		on,
		label,
		title,
		words,
		shown = false,
		disabled = false,
		onchange,
		...rest
	}: {
		on: boolean;
		label: string;
		title?: string;
		words?: readonly [off: string, on: string];
		shown?: boolean;
		disabled?: boolean;
		onchange: (on: boolean) => void;
		id?: string;
		'data-kit-state'?: string;
		'data-testid'?: string;
	} = $props();

	const keydown = (e: KeyboardEvent) => {
		const what = switchKeyDown(e.key, e.repeat);
		if (what === 'none') return;
		e.preventDefault();
		if (what === 'flip' && !disabled) onchange(!on);
	};
	const keyup = (e: KeyboardEvent) => {
		if (isSwitchKey(e.key)) e.preventDefault();
	};
</script>

<button
	{...rest}
	type="button"
	class="switch"
	role="switch"
	aria-checked={on}
	aria-label={label}
	title={title ?? label}
	{disabled}
	onclick={() => onchange(!on)}
	onkeydown={keydown}
	onkeyup={keyup}
	><span class="track" aria-hidden="true"></span>{#if shown}<span class="shown" aria-hidden="true">{label}</span>{/if}{#if words}<span class="value" aria-hidden="true">{on ? words[1] : words[0]}</span>{/if}</button
>

<style>
	.switch {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: 0.375rem;
		height: var(--control-height);
		min-width: var(--control-height);
		padding: 0;
		border: none;
		background: none;
		color: var(--tool-text);
		font: inherit;
		cursor: pointer;
	}
	/* deck-kit controls.rs `switch`: a round track with its line, the knob 0.76 of its height; on (no green, RULES H11), the
	   track takes the pressed fill and the knob right in the pressed text (kit.css --kit-toggle-*) */
	.track {
		position: relative;
		flex: none;
		box-sizing: border-box;
		width: 2.25rem;
		height: 1.125rem;
		border: var(--editor-edge) solid var(--kit-toggle-off-border);
		border-radius: 999px;
		background: var(--kit-toggle-off-ground);
		transition:
			background-color 0.15s,
			border-color 0.15s;
	}
	.track::before {
		content: '';
		position: absolute;
		top: 50%;
		left: 0.07rem;
		width: 0.855rem;
		height: 0.855rem;
		border-radius: 50%;
		background: var(--kit-toggle-off-knob);
		transform: translateY(-50%);
		transition:
			transform 0.15s,
			background-color 0.15s;
	}
	.switch[data-kit-state~='hover'] .track {
		border-color: var(--kit-toggle-hover-border);
	}
	@media (hover: hover) {
		.switch:hover:not(:disabled) .track {
			border-color: var(--kit-toggle-hover-border);
		}
	}
	.switch[aria-checked='true'] .track {
		border-color: var(--kit-toggle-on-border);
		background: var(--kit-toggle-on-ground);
	}
	.switch[aria-checked='true'] .track::before {
		background: var(--kit-toggle-on-knob);
		transform: translate(1.125rem, -50%);
	}
	.switch:focus-visible {
		outline: none;
	}
	.switch:focus-visible .track,
	.switch[data-kit-state~='focus'] .track {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.switch:disabled {
		opacity: var(--kit-toggle-disabled-opacity);
		cursor: not-allowed;
	}
	.shown {
		min-width: 0;
		overflow: hidden;
		text-align: start;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.value {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	@media (prefers-reduced-motion: reduce) {
		.track,
		.track::before {
			transition: none;
		}
	}
</style>
