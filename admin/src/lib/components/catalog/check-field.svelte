<script lang="ts">
	/** A true / false catalog field: a checkbox with its label on one line */
	let { label, checked, onchange, testid }: { label: string; checked: boolean; onchange: (checked: boolean) => void; testid?: string } = $props();
</script>

<label class="check">
	<input type="checkbox" {checked} onchange={(e) => onchange(e.currentTarget.checked)} data-testid={testid} />
	<span>{label}</span>
</label>

<style>
	.check {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
	}
</style>
