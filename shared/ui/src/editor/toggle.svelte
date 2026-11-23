<script lang="ts">
	import { isSwitchKey, switchKeyDown } from './toggle-keys.ts';

	/**
	 * The editor's on/off switch: a track with a knob that slides right and fills the track while on – a
	 * `<button role="switch">`, Space and Enter flip it once per press. Every boolean of a panel is this switch, never a
	 * checkbox or an on/off segmented control. The row's word is its visible label; `label` names it for assistive
	 * technology; `words` adds a quiet value after the track where the value has words of its own.
	 */
	let {
		on,
		label,
		title,
		words,
		disabled = false,
		onchange
	}: {
		on: boolean;
		label: string;
		title?: string;
		words?: readonly [off: string, on: string];
		disabled?: boolean;
		onchange: (on: boolean) => void;
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
	><span class="track" aria-hidden="true"></span>{#if words}<span class="value" aria-hidden="true">{on ? words[1] : words[0]}</span>{/if}</button
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
	.track {
		position: relative;
		flex: none;
		box-sizing: border-box;
		width: 1.875rem;
		height: 1.125rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 999px;
		background: var(--tool-control);
		transition:
			background-color 0.15s,
			border-color 0.15s;
	}
	.track::before {
		content: '';
		position: absolute;
		top: 50%;
		left: 0.125rem;
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 50%;
		background: var(--tool-muted);
		transform: translateY(-50%);
		transition:
			transform 0.15s,
			background-color 0.15s;
	}
	.switch[aria-checked='true'] .track {
		border-color: var(--tool-text);
		background: var(--tool-text);
	}
	.switch[aria-checked='true'] .track::before {
		background: var(--tool-pressed-text);
		transform: translate(0.75rem, -50%);
	}
	@media (hover: hover) {
		.switch:hover:not(:disabled) .track {
			border-color: var(--tool-muted);
		}
		.switch[aria-checked='true']:hover:not(:disabled) .track {
			border-color: var(--tool-text);
		}
	}
	.switch:focus-visible {
		outline: none;
	}
	.switch:focus-visible .track {
		outline: 2px solid var(--tool-accent);
		outline-offset: 1px;
	}
	.switch:disabled {
		opacity: 0.45;
		cursor: not-allowed;
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
