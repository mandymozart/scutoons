<script lang="ts" module>
	/** `group`: options in a row with the same group are listed under its head (an `<optgroup>`) */
	export type SelectOption = { value: string; label: string; disabled?: boolean; group?: string };
</script>

<script lang="ts">
	import type { HTMLSelectAttributes } from 'svelte/elements';

	/**
	 * The kit's select (RULES H4): boxed – a value set in a form, the field's look with its chevron – or `quiet` – a filter
	 * or a sort in a toolbar: no box at rest, a rounded border on hover and while open (as a tab). Open turns the chevron up
	 * and the line to the text colour. A native `<select>` underneath, so the keys and the list are the platform's. The
	 * states' colours are kit.css's `--kit-field-*` / `--kit-select-quiet-*`.
	 */
	type Props = {
		value?: string;
		options: readonly SelectOption[];
		label: string;
		quiet?: boolean;
		disabled?: boolean;
		onchange?: (value: string) => void;
		'data-kit-state'?: string;
	} & Omit<HTMLSelectAttributes, 'value' | 'onchange'>;

	let { value = $bindable(''), options, label, quiet = false, disabled = false, onchange, 'data-kit-state': kitState, ...rest }: Props = $props();
	let open = $state(false);
	/** the options in runs of one group (no group: a run of its own, without a head) */
	const runs = $derived(
		options.reduce<{ group?: string; options: SelectOption[] }[]>((out, o) => {
			const last = out.at(-1);
			if (last && o.group && last.group === o.group) last.options.push(o);
			else out.push({ group: o.group, options: [o] });
			return out;
		}, [])
	);
</script>

<span class="kit-select" class:quiet class:open class:disabled data-kit-state={kitState}>
	<select
		{...rest}
		aria-label={label}
		title={rest.title ?? label}
		{disabled}
		bind:value
		onchange={() => {
			open = false;
			onchange?.(value);
		}}
		onmousedown={() => (open = !open)}
		onblur={() => (open = false)}
		onkeydown={(e) => {
			if (e.key === 'Escape' || e.key === 'Enter' || e.key === 'Tab') open = false;
		}}
	>
		{#each runs as run, i (i)}
			{#if run.group}
				<optgroup label={run.group}>{#each run.options as o (o.value)}<option value={o.value} disabled={o.disabled}>{o.label}</option>{/each}</optgroup>
			{:else}
				{#each run.options as o (o.value)}<option value={o.value} disabled={o.disabled}>{o.label}</option>{/each}
			{/if}
		{/each}
	</select>
</span>

<style>
	.kit-select {
		position: relative;
		display: inline-flex;
		flex: none;
		align-items: center;
		box-sizing: border-box;
		min-width: 0;
		height: var(--kit-h, var(--control-height));
		border: var(--editor-edge) solid var(--kit-field-rest-border);
		border-radius: var(--editor-radius);
		background: var(--kit-field-rest-ground);
		color: var(--kit-field-rest-text);
		text-shadow: none;
	}
	/* the chevron (kit-d.html): two lines, down at rest, up while open */
	.kit-select::after {
		content: '';
		position: absolute;
		right: var(--editor-pad);
		top: 50%;
		width: 0.45rem;
		height: 0.45rem;
		border-right: 1.5px solid var(--kit-field-rest-mark);
		border-bottom: 1.5px solid var(--kit-field-rest-mark);
		transform: translateY(-70%) rotate(45deg);
		pointer-events: none;
	}
	select {
		box-sizing: border-box;
		width: 100%;
		height: 100%; /* control-ok: fills the kit select's box, which is a control high */
		min-height: 0;
		margin: 0;
		padding: 0 calc(var(--editor-pad) * 2 + 0.45rem) 0 var(--editor-pad);
		border: 0;
		border-radius: inherit;
		background: none;
		color: inherit;
		font: inherit;
		appearance: none;
		cursor: pointer;
	}
	select:focus {
		outline: none;
	}
	select option,
	select optgroup {
		background: var(--tool-ground);
		color: var(--tool-text);
	}
	select optgroup {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-style: normal;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	select optgroup option {
		color: var(--tool-text);
		font-size: var(--editor-small, 0.875rem);
		letter-spacing: normal;
		text-transform: none;
	}
	.kit-select[data-kit-state~='hover'] {
		border-color: var(--kit-field-hover-border);
	}
	@media (hover: hover) {
		.kit-select:hover:not(.disabled) {
			border-color: var(--kit-field-hover-border);
		}
	}
	.open,
	.kit-select[data-kit-state~='on'] {
		border-color: var(--kit-field-on-border);
	}
	.open::after,
	.kit-select[data-kit-state~='on']::after {
		border-color: var(--kit-field-on-mark);
		transform: translateY(-20%) rotate(225deg);
	}
	.kit-select:has(select:focus-visible),
	.kit-select[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.disabled {
		color: var(--kit-field-disabled-text);
		opacity: var(--kit-field-disabled-opacity);
	}
	.disabled select {
		cursor: not-allowed;
	}
	/* quiet: a filter or a sort – no box at rest, round as a tab, a border on hover and while open */
	.quiet {
		border-color: var(--kit-select-quiet-rest-border);
		border-radius: calc(var(--kit-h, var(--control-height)) / 2);
		background: var(--kit-select-quiet-rest-ground);
		color: var(--kit-select-quiet-rest-text);
	}
	.quiet::after {
		border-color: var(--kit-select-quiet-rest-mark);
	}
	.quiet select {
		field-sizing: content;
		padding-left: calc(var(--editor-pad) * 1.25);
	}
	.quiet[data-kit-state~='hover'] {
		border-color: var(--kit-select-quiet-hover-border);
	}
	@media (hover: hover) {
		.quiet:hover:not(.disabled) {
			border-color: var(--kit-select-quiet-hover-border);
		}
	}
	.quiet.open,
	.quiet[data-kit-state~='on'] {
		border-color: var(--kit-select-quiet-on-border);
	}
	.quiet.open::after,
	.quiet[data-kit-state~='on']::after {
		border-color: var(--kit-select-quiet-on-mark);
	}
	.quiet.disabled {
		opacity: var(--kit-select-quiet-disabled-opacity);
	}
</style>
