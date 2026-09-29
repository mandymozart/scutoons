<script lang="ts">
	/**
	 * On/off switch (role="switch"). Square track, ink border; on = primary-cyan fill (fill only, never text).
	 */
	let {
		checked = $bindable(false),
		label,
		hint,
		disabled = false,
		onchange
	}: {
		checked?: boolean;
		label: string;
		hint?: string;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
	} = $props();

	const uid = $props.id();

	function toggle() {
		if (disabled) return;
		checked = !checked;
		onchange?.(checked);
	}
</script>

<div class="sc-toggle" class:disabled>
	<button
		type="button"
		role="switch"
		aria-checked={checked}
		aria-labelledby="{uid}-label"
		aria-describedby={hint ? `${uid}-hint` : undefined}
		{disabled}
		onclick={toggle}
	>
		<span class="knob"></span>
	</button>
	<span class="text">
		<span id="{uid}-label" class="label">{label}</span>
		{#if hint}<span id="{uid}-hint" class="hint">{hint}</span>{/if}
	</span>
</div>

<style>
	.sc-toggle {
		display: flex;
		align-items: flex-start;
		gap: var(--space-075);
	}
	button {
		flex: none;
		position: relative;
		box-sizing: border-box;
		width: 2.5em;
		height: 1.25em;
		margin: 0.1em 0 0;
		padding: 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		cursor: pointer;
		transition: background-color var(--duration-fast) var(--easing);
	}
	button[aria-checked='true'] {
		background: var(--primary-cyan);
	}
	.knob {
		position: absolute;
		top: 2px;
		left: 2px;
		width: calc(1.25em - 8px);
		height: calc(1.25em - 8px);
		background: var(--ink);
		transition: transform var(--duration-fast) var(--easing);
	}
	button[aria-checked='true'] .knob {
		transform: translateX(1.25em);
	}
	button:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
	.text {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.hint {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.disabled {
		opacity: 0.5;
	}
	.disabled button {
		cursor: not-allowed;
	}
</style>
