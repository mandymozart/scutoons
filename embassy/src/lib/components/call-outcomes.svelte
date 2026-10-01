<script lang="ts">
	import { Button } from '@scutoons/ui';
	import { CALL_OUTCOMES, type CallOutcome } from '@scutoons/api';
	import { addDays, label, todayIso } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The buttons after a call: one tap logs it (the server sets the next date); "call back …" asks for the date first.
	 */
	let {
		disabled = false,
		onlog
	}: { disabled?: boolean; onlog: (outcome: CallOutcome, nextAt?: string) => void } = $props();

	let askDate = $state(false);
	let date = $state(addDays(todayIso(), 2));
</script>

<div class="outcomes">
	{#if askDate}
		<form
			class="row"
			onsubmit={(e) => {
				e.preventDefault();
				askDate = false;
				onlog('call_back', date);
			}}
		>
			<label class="field">
				<span>{m['today.call_back_on']()}</span>
				<input type="date" bind:value={date} min={todayIso()} required />
			</label>
			<Button type="submit" size="sm" {disabled}>{m['today.save']()}</Button>
			<Button variant="clear" size="sm" onclick={() => (askDate = false)}>{m['org.cancel']()}</Button>
		</form>
	{:else}
		{#each CALL_OUTCOMES as outcome (outcome)}
			<Button
				size="sm"
				variant={outcome === 'reached' ? 'primary' : 'secondary'}
				{disabled}
				onclick={() => (outcome === 'call_back' ? (askDate = true) : onlog(outcome))}
			>
				{label('outcome', outcome)}
			</Button>
		{/each}
	{/if}
</div>

<style>
	.outcomes {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.outcomes form {
		align-items: flex-end;
	}
</style>
