<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { ActionBar, Button, Field, Section, Skeleton } from '@scutoons/ui';
	import type { AdminSetting, GenerationBudgetState } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/** Runtime settings of the API (GET/PUT /api/admin/settings): the rate limits and the generation budget (#121). */
	const toaster = getToaster();

	// the explanations of the limits are in the Info panel (PageHelp "settings")
	const LABELS: Record<string, { label: () => string }> = {
		scan_limit_per_hour: { label: () => m['settings.scan_limit_per_hour']() },
		lookup_limit_per_hour: { label: () => m['settings.lookup_limit_per_hour']() },
		newsletter_per_hour: { label: () => m['settings.newsletter_per_hour']() },
		newsletter_batch: { label: () => m['settings.newsletter_batch']() },
		generation_budget: { label: () => m['settings.generation_budget']() }
	};
	const BUDGET = 'generation_budget';

	let items = $state<AdminSetting[] | null>(null);
	/** spent / running / left of the generation budget (read-only, in euro) */
	let budget = $state<GenerationBudgetState | null>(null);
	/** the form: key → value as typed (number inputs give numbers, empty → null) */
	let draft = $state<Record<string, number | null>>({});
	let error = $state<string | null>(null);
	let saving = $state(false);

	function load() {
		error = null;
		admin
			.settings()
			.then((r) => apply(r.items, r.budget))
			.catch((e) => (error = errorText(e)));
	}

	function apply(next: AdminSetting[], state?: GenerationBudgetState) {
		items = next;
		budget = state ?? null;
		draft = Object.fromEntries(next.map((s) => [s.key, s.value]));
	}

	load();

	function invalid(s: AdminSetting): boolean {
		const v = draft[s.key];
		return typeof v !== 'number' || !Number.isInteger(v) || v < s.min || v > s.max;
	}

	const changed = $derived((items ?? []).filter((s) => draft[s.key] !== s.value));
	const anyInvalid = $derived((items ?? []).some(invalid));

	async function save() {
		if (!items || anyInvalid) return;
		if (changed.length === 0) {
			toaster.addToast({ message: m['settings.nothing_changed'](), type: 'info' });
			return;
		}
		saving = true;
		try {
			const result = await admin.saveSettings(Object.fromEntries(changed.map((s) => [s.key, draft[s.key]])));
			apply(result.items, result.budget);
			toaster.addToast({ message: m['settings.saved'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	const limits = $derived((items ?? []).filter((s) => s.key !== BUDGET));
	const budgetSetting = $derived((items ?? []).find((s) => s.key === BUDGET) ?? null);
	const valueText = (s: AdminSetting, v: number) =>
		s.key === BUDGET ? (v === 0 ? m['settings.no_budget']() : euro(v)) : v === 0 ? m['settings.unlimited']() : String(v);
	const euro = (v: number) => new Intl.NumberFormat(getLocale(), { style: 'currency', currency: 'EUR' }).format(v);
</script>

<PageHeader type={m['nav.group_system']()} title={m['settings.title']()} meta={m['settings.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['settings.title']()}>
	{#snippet info()}<PageHelp page="settings" />{/snippet}
	{#snippet start()}
		{#if changed.length > 0}<span class="unsaved">{m['settings.unsaved']()}</span>{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" onclick={save} disabled={!items || saving || anyInvalid || changed.length === 0}>
			{saving ? m['settings.saving']() : m['settings.save']()}
		</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !items}
	<Skeleton lines={6} label={m['shell.loading']()} />
{:else}
	<form
		onsubmit={(event) => {
			event.preventDefault();
			save();
		}}
	>
		<Section title={m['settings.section_limits']()}>
			<div class="settings">
				{#each limits as s (s.key)}{@render setting(s)}{/each}
			</div>
		</Section>
		{#if budgetSetting}
			<Section title={m['settings.section_budget']()}>
				<div class="settings">
					{@render setting(budgetSetting)}
					{#if budget}
						<dl class="budget">
							<dt>{m['settings.budget_spent']()}</dt>
							<dd>{euro(budget.spent)}</dd>
							<dt>{m['settings.budget_reserved']()}</dt>
							<dd>{euro(budget.reserved)}</dd>
							<dt>{m['settings.budget_left']()}</dt>
							<dd>{euro(budget.left)}</dd>
						</dl>
						<p class="state">
							{budget.rate.source === 'ecb'
								? m['settings.budget_rate']({ rate: budget.rate.eur_per_usd.toFixed(4), date: budget.rate.date })
								: m['settings.budget_rate_fallback']({ rate: budget.rate.eur_per_usd.toFixed(4), date: budget.rate.date })}
						</p>
					{/if}
				</div>
			</Section>
		{/if}
		<button type="submit" hidden>{m['settings.save']()}</button>
	</form>
{/if}

{#snippet setting(s: AdminSetting)}
	<div class="setting">
		<Field label={LABELS[s.key]?.label() ?? s.key} error={invalid(s) ? m['settings.invalid']({ min: s.min, max: s.max }) : null}>
			{#snippet children({ id, describedby, invalid: isInvalid })}
				<div class="control">
					<input
						{id}
						name={s.key}
						type="number"
						inputmode="numeric"
						min={s.min}
						max={s.max}
						step="1"
						aria-describedby={describedby}
						aria-invalid={isInvalid}
						bind:value={draft[s.key]}
					/>
					{#if draft[s.key] !== s.default}
						<Button size="sm" variant="secondary" onclick={() => (draft[s.key] = s.default)}>{m['settings.reset']()}</Button>
					{/if}
				</div>
			{/snippet}
		</Field>
		<p class="state">
			<span>{m['settings.current']({ value: valueText(s, s.value), default: valueText(s, s.default) })}</span>
			<span>
				{s.stored && s.updated_at
					? m['settings.last_change']({ at: minute(s.updated_at), by: (s.updated_by ?? '—').replace(/^admin:/, '') })
					: m['settings.never_changed']()}
			</span>
		</p>
	</div>
{/snippet}

<style>
	.settings {
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
		max-width: 44rem;
	}
	.control {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.control input {
		width: 10rem;
	}
	.control :global(.sc-button) {
		white-space: nowrap;
	}
	.state {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-15);
		margin: 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	.unsaved {
		color: var(--text-muted);
	}
	.budget {
		display: grid;
		grid-template-columns: max-content max-content;
		gap: var(--space-05) var(--space-15);
		margin: 0;
	}
	.budget dt {
		color: var(--text-muted);
	}
	.budget dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}
</style>
