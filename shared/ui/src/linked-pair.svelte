<script lang="ts" module>
	import type { Units } from './units.ts';
	export interface PairField {
		/** the accessible name and the tooltip */
		label: string;
		tag?: string;
		icon?: string;
		placeholder?: string;
		required?: boolean;
		disabled?: boolean;
		/** the field's units (units.ts; W and H can differ – the slice's width / height) */
		units?: Units;
		testid?: string;
	}
</script>

<script lang="ts">
	import Icon from './icon.svelte';
	import ScrubNumber from './scrub-number.svelte';

	/**
	 * Two numbers that can be linked (agents/research/EDITOR-INPUTS.md; W ⟷ H): two ScrubNumbers and a square lock
	 * toggle between them (grouped, so square – DESIGN.md "Round vs square"). Locked, a change of one scales the other by
	 * their ratio; the lock is off while one of them is unset (auto). `onchange` gets both values, so the app writes them
	 * as one edit (one undo step).
	 */
	let {
		a,
		b,
		first,
		second,
		onchange,
		lockLabel,
		locked = $bindable(false),
		unit,
		step = 1,
		min,
		max
	}: {
		a: number | null | undefined;
		b: number | null | undefined;
		first: PairField;
		second: PairField;
		onchange: (a: number | undefined, b: number | undefined) => void;
		/** the lock's accessible name and tooltip ("Keep proportions") */
		lockLabel: string;
		locked?: boolean;
		unit?: string;
		step?: number;
		min?: number;
		max?: number;
	} = $props();

	const linkable = $derived(typeof a === 'number' && typeof b === 'number' && a !== 0 && b !== 0);
	const on = $derived(locked && linkable);
	const round = (v: number) => Math.round(v * 100) / 100;

	function setA(next: number | undefined) {
		if (on && next !== undefined) onchange(next, round((b as number) * (next / (a as number))));
		else onchange(next, b ?? undefined);
	}
	function setB(next: number | undefined) {
		if (on && next !== undefined) onchange(round((a as number) * (next / (b as number))), next);
		else onchange(a ?? undefined, next);
	}
</script>

<span class="sc-linked-pair">
	<ScrubNumber value={a} onchange={setA} {...first} {unit} {step} {min} {max} />
	<button
		type="button"
		class="lock"
		aria-pressed={on}
		aria-label={lockLabel}
		title={lockLabel}
		disabled={!linkable}
		onclick={() => (locked = !locked)}
	>
		<Icon name={on ? 'lock' : 'unlock'} size="1.1em" />
	</button>
	<ScrubNumber value={b} onchange={setB} {...second} {unit} {step} {min} {max} />
</span>

<style>
	.sc-linked-pair {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
	}
	.sc-linked-pair > :global(.sc-scrub) {
		flex: 1 1 0;
	}
	.lock {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		padding: 0;
		border: var(--border-width-regular) solid transparent;
		background: transparent;
		color: var(--text-muted);
	}
	.lock:hover:not(:disabled) {
		border-color: var(--border-muted);
		color: var(--ink);
	}
	.lock[aria-pressed='true'] {
		border-color: var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
	.lock:disabled {
		opacity: 0.5;
	}
</style>
