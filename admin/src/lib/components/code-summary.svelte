<script lang="ts">
	import { Skeleton } from '@scutoons/ui';
	import type { AdminCodeDetail, CodeEvent } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, errorText, minute, number } from '$lib/format.ts';
	import { eventLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CodeStatus from '$lib/components/code-status.svelte';
	import RevokeForm from '$lib/components/revoke-form.svelte';

	/** Triage view of one code (drawer of the codes list): status, facts, activity, revoke, the latest events. */
	let { code, onchanged }: { code: string; onchanged?: () => void } = $props();

	let detail = $state<AdminCodeDetail | null>(null);
	let events = $state<CodeEvent[]>([]);
	let error = $state<string | null>(null);

	$effect(() => {
		const current = code;
		detail = null;
		error = null;
		Promise.all([admin.getCode(current), admin.codeEvents(current, 10)])
			.then(([d, e]) => {
				if (current !== code) return;
				detail = d;
				events = e.items;
			})
			.catch((e) => (error = errorText(e)));
	});
</script>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !detail}
	<Skeleton lines={8} label={m['shell.loading']()} />
{:else}
	{@const c = detail.code}
	<div class="summary">
		<CodeStatus claimed={c.claimed} revoked={detail.revoked} expired={detail.expired} signed={detail.signed} />
		<dl class="facts">
			<dt>{m['codes.col_type']()}</dt>
			<dd>{c.type}</dd>
			<dt>{m['common.edition']()}</dt>
			<dd>{detail.edition ? `${detail.edition.name} #${c.edition_number ?? '—'}` : m['common.no_edition']()}</dd>
			<dt>{m['codes.created']()}</dt>
			<dd>{minute(c.created_at)}</dd>
			<dt>{m['codes.col_claimed']()}</dt>
			<dd>{c.claimed ? minute(c.claimed_at) : '—'}</dd>
			<dt>{m['codes.expires']()}</dt>
			<dd>{c.expires_at ? minute(c.expires_at) : m['codes.no_expiry']()}</dd>
			<dt>{m['codes.redirect']()}</dt>
			<dd>{c.redirect_to}</dd>
			<dt>{m['codes.activity']()}</dt>
			<dd>
				{m['codes.activity_line']({
					scans: number(detail.stats.scans),
					devices: number(detail.stats.devices),
					accesses: number(detail.stats.accesses),
					logins: number(detail.stats.logins)
				})}
			</dd>
		</dl>
		<RevokeForm
			code={c.code}
			revoked={detail.revoked}
			reason={c.revoked_reason}
			onchanged={(state) => {
				if (!detail) return;
				detail.code.revoked_at = state.revoked_at;
				detail.code.revoked_reason = state.revoked_reason;
				detail.revoked = !!state.revoked_at;
				onchanged?.();
			}}
		/>
		<h3>{m['codes.latest_events']()}</h3>
		{#if events.length}
			<ol class="events">
				{#each events as e, i (i)}
					<li><span class="muted">{minute(e.created_at)}</span> {eventLabel(e.event)}{#if e.account} · {m['events.with_account']()}{/if}</li>
				{/each}
			</ol>
		{:else}
			<p class="muted">{m['codes.no_events']()}</p>
		{/if}
		<p class="muted">{m['codes.first_last']({ first: day(detail.stats.first_seen), last: day(detail.stats.last_seen) })}</p>
	</div>
{/if}

<style>
	.summary {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	h3 {
		margin: 0;
	}
	.events {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.summary > p {
		margin: 0;
	}
</style>
