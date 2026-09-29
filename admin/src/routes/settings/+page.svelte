<script lang="ts">
	import { ActionBar, Button, Field, Section, Skeleton } from '@scutoons/ui';
	import type { AdminSetting } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/** Runtime settings of the API (GET/PUT /api/admin/settings): the scan and lookup rate limits. */
	const toaster = getToaster();

	const LABELS: Record<string, { label: () => string; hint: () => string }> = {
		scan_limit_per_hour: { label: () => m['settings.scan_limit_per_hour'](), hint: () => m['settings.scan_limit_per_hour_hint']() },
		lookup_limit_per_hour: { label: () => m['settings.lookup_limit_per_hour'](), hint: () => m['settings.lookup_limit_per_hour_hint']() }
	};

	let items = $state<AdminSetting[] | null>(null);
	/** the form: key → value as typed (number inputs give numbers, empty → null) */
	let draft = $state<Record<string, number | null>>({});
	let error = $state<string | null>(null);
	let saving = $state(false);

	function load() {
		error = null;
		admin
			.settings()
			.then((r) => apply(r.items))
			.catch((e) => (error = errorText(e)));
	}

	function apply(next: AdminSetting[]) {
		items = next;
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
			apply(result.items);
			toaster.addToast({ message: m['settings.saved'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	const valueText = (v: number) => (v === 0 ? m['settings.unlimited']() : String(v));
</script>

<PageHeader type={m['nav.group_system']()} title={m['settings.title']()} meta={m['settings.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()}>
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
	<Section title={m['settings.section_limits']()} hint={m['settings.section_limits_hint']()}>
		<form
			class="settings"
			onsubmit={(event) => {
				event.preventDefault();
				save();
			}}
		>
			{#each items as s (s.key)}
				<div class="setting">
					<Field
						label={LABELS[s.key]?.label() ?? s.key}
						hint={LABELS[s.key]?.hint()}
						error={invalid(s) ? m['settings.invalid']({ min: s.min, max: s.max }) : null}
					>
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
						<span>{m['settings.current']({ value: valueText(s.value), default: valueText(s.default) })}</span>
						<span>
							{s.stored && s.updated_at
								? m['settings.last_change']({ at: minute(s.updated_at), by: (s.updated_by ?? '—').replace(/^admin:/, '') })
								: m['settings.never_changed']()}
						</span>
					</p>
				</div>
			{/each}
			<button type="submit" hidden>{m['settings.save']()}</button>
		</form>
	</Section>
{/if}

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
</style>
