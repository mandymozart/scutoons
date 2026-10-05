<script lang="ts">
	// Contacts per week and the reminder mail – the onboarding's two choices, also on Today to change them any time
	// (owner 2026-10-05: "a preset and adjustability on their own pace"). Presets 5 / 10 / 15 / 20, "other" = any number.
	import { Segmented } from '@scutoons/ui';
	import { PACE_REMINDERS, type PaceReminder } from '@scutoons/api';
	import { m } from '$lib/paraglide/messages.js';

	let { perWeek = $bindable(), reminder = $bindable(), compact = false }: { perWeek: number; reminder: PaceReminder; compact?: boolean } = $props();

	const PRESETS = ['5', '10', '15', '20'];
	const REMINDER_LABELS: Record<PaceReminder, () => string> = { weekly: m['pace.weekly'], daily: m['pace.daily'], off: m['pace.off'] };
	// svelte-ignore state_referenced_locally
	let other = $state(!PRESETS.includes(String(perWeek)));
	const chosen = $derived(other ? 'other' : String(perWeek));
	// ~12 minutes a contact (a short mail or call, logging included), in half hours
	const hours = $derived(Math.max(0.5, Math.round(perWeek * 0.2 * 2) / 2));
</script>

<div class="pace" class:compact>
	<div>
		<p class="lbl">{m['pace.per_week']()}</p>
		<div class="row">
			<Segmented
				label={m['pace.per_week']()}
				options={[...PRESETS.map((v) => ({ value: v, label: v })), { value: 'other', label: m['pace.other']() }]}
				value={chosen}
				onchange={(v) => (v === 'other' ? (other = true) : ((other = false), (perWeek = Number(v))))}
			/>
			{#if other}<input type="number" min="1" max="100" bind:value={perWeek} aria-label={m['pace.per_week']()} />{/if}
		</div>
		{#if !compact}<p class="caption muted">{m['pace.time']({ hours: String(hours) })}</p>{/if}
	</div>
	<div>
		<p class="lbl">{m['pace.reminder']()}</p>
		<Segmented label={m['pace.reminder']()} options={PACE_REMINDERS.map((r) => ({ value: r, label: REMINDER_LABELS[r]() }))} value={reminder} onchange={(r) => (reminder = r)} />
	</div>
</div>

<style>
	.pace {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.lbl {
		margin: 0 0 var(--space-025);
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
	}
	input {
		width: 5rem;
	}
	.caption {
		margin: var(--space-025) 0 0;
		font-size: var(--type-caption-size);
	}
	.muted {
		color: var(--text-muted);
	}
</style>
