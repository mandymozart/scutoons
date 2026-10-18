<script lang="ts">
	import Icon from './icon.svelte';
	import { uiLabels } from './ui-labels.ts';

	/**
	 * The reset button of a field (owner 2026-10-18: the "×" behind fields wasn't clear – one look, one place, everywhere):
	 * inside the field's frame at its right end, the full control height, a Feather counter-clockwise arrow (not a cross –
	 * a cross closes or removes), "Reset" / "Zurücksetzen" as tooltip and name. The field shows it only while a value is
	 * set; a click sets it back to its default (the placeholder) in one step. ScrubNumber, ColorField and PropSelect use it.
	 */
	let { onreset, label, testid }: { onreset: () => void; /** default: the app's reset word (ui-labels.ts) */ label?: string; testid?: string } = $props();

	const t = uiLabels();
</script>

<button
	type="button"
	class="sc-field-reset"
	aria-label={label ?? t('reset')}
	title={label ?? t('reset')}
	data-testid={testid}
	onpointerdown={(e) => e.stopPropagation()}
	onclick={(e) => {
		e.stopPropagation();
		onreset();
	}}
>
	<Icon name="rotate-ccw" size="1em" />
</button>

<style>
	.sc-field-reset {
		position: absolute;
		inset: 0 0 0 auto;
		z-index: 2;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: var(--field-reset-width, 1.5rem);
		margin: 0;
		padding: 0;
		border: var(--border-width-hairline) solid transparent;
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.sc-field-reset:hover {
		color: var(--ink);
		border-color: var(--ink);
	}
</style>
