<script lang="ts">
	import { Button, Field, Sheet, confirmDialog } from '@scutoons/ui';
	import { craftName, type Craft, type MemberInvite } from '@scutoons/api';
	import { admin, crafts as craftsApi } from '$lib/api.ts';
	import { day, errorText } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import CraftChips from '$lib/components/craft-chips.svelte';

	/** Team invites (docs/API.md "Crafts"): an address and the crafts its member starts with, given when its account comes (an invite signs no one up). */
	let { open = $bindable(false) }: { open?: boolean } = $props();

	const toaster = getToaster();
	let crafts = $state<Craft[]>([]);
	let invites = $state<MemberInvite[]>([]);
	let email = $state('');
	let picked = $state<string[]>([]);
	let working = $state(false);

	$effect(() => {
		if (!open) return;
		Promise.all([craftsApi.list(), admin.memberInvites()])
			.then(([c, i]) => ((crafts = c.crafts), (invites = i.items)))
			.catch((e) => toaster.addToast({ message: errorText(e), type: 'error' }));
	});

	const names = (ids: string[]) => ids.map((id) => crafts.find((c) => c.id === id)).filter((c): c is Craft => !!c).map((c) => craftName(c, getLocale())).join(' · ');

	async function invite() {
		if (!email.trim()) return;
		working = true;
		try {
			const r = await admin.inviteMember(email.trim(), picked);
			invites = [r.invite, ...invites.filter((i) => i.id !== r.invite.id)];
			toaster.addToast({ message: m['invites.sent']({ email: r.invite.email }), type: 'success' });
			email = '';
			picked = [];
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			working = false;
		}
	}

	async function withdraw(i: MemberInvite) {
		if (!(await confirmDialog({ title: m['invites.withdraw_title']({ email: i.email }), message: m['invites.withdraw_message'](), confirmLabel: m['invites.withdraw']() }))) return;
		try {
			await admin.withdrawMemberInvite(i.id);
			invites = invites.filter((x) => x.id !== i.id);
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		}
	}
</script>

<Sheet bind:open title={m['invites.title']()} side="right" closeLabel={m['common.close']()}>
	<form class="form" onsubmit={(e) => (e.preventDefault(), invite())}>
		<Field label={m['invites.email']()}>
			{#snippet children({ id })}
				<input {id} type="email" bind:value={email} autocomplete="off" required />
			{/snippet}
		</Field>
		<div class="field-label">{m['invites.crafts']()}</div>
		<CraftChips {crafts} selected={picked} disabled={working} onchange={(next) => (picked = next)} />
	</form>
	{#if invites.length}
		<ul class="list">
			{#each invites as i (i.id)}
				<li>
					<span class="who">{i.email}</span>
					<span class="muted">{names(i.crafts) || '—'}</span>
					<span class="muted">{i.accepted_at ? m['invites.accepted']({ date: day(i.accepted_at) }) : m['invites.open']({ date: day(i.created_at) })}</span>
					{#if i.accepted_at && i.user_id}
						<a href="/users/{encodeURIComponent(i.user_id)}" onclick={() => (open = false)}>{m['invites.user']()}</a>
					{:else if !i.accepted_at}
						<Button size="sm" variant="secondary" onclick={() => withdraw(i)}>{m['invites.withdraw']()}</Button>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
	{#snippet footer()}
		<Button onclick={invite} disabled={working || !email.trim()}>{m['invites.send']()}</Button>
	{/snippet}
</Sheet>

<style>
	.form {
		display: grid;
		gap: var(--space-05);
	}
	.field-label {
		margin-top: var(--space-05);
	}
	.list {
		display: grid;
		gap: var(--space-05);
		margin: var(--space-2) 0 0;
		padding: 0;
		list-style: none;
	}
	.list li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
	}
	.who {
		margin-right: auto;
	}
</style>
