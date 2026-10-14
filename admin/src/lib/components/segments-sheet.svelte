<script lang="ts">
	import { Button, Sheet, askDialog, confirmDialog } from '@scutoons/ui';
	import type { AdminSegment } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { campaignError } from '$lib/campaigns.ts';
	import { number } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The segments (agents/research/GIFT-INVITES.md 1b): named groups of accounts – make, rename, delete. Members are
	 * added on the Users list (ticked accounts or all matching); `onchange` after every change.
	 */
	let {
		open = $bindable(false),
		segments,
		onchange
	}: {
		open?: boolean;
		segments: AdminSegment[];
		onchange: () => void;
	} = $props();

	const toaster = getToaster();

	async function run(action: () => Promise<unknown>) {
		try {
			await action();
			onchange();
		} catch (e) {
			toaster.addToast({ message: campaignError(e), type: 'error' });
		}
	}

	async function create() {
		const name = await askDialog({ title: m['segments.new'](), label: m['segments.name'](), placeholder: m['segments.name_hint'](), confirmLabel: m['segments.create']() });
		if (name?.trim()) await run(() => admin.createSegment(name.trim()));
	}

	async function rename(s: AdminSegment) {
		const name = await askDialog({ title: m['segments.rename'](), label: m['segments.name'](), placeholder: s.name, confirmLabel: m['segments.save']() });
		if (name?.trim() && name.trim() !== s.name) await run(() => admin.renameSegment(s.id, name.trim()));
	}

	async function remove(s: AdminSegment) {
		const ok = await confirmDialog({ title: m['segments.delete_title']({ name: s.name }), message: m['segments.delete_message']({ count: number(s.members) }), confirmLabel: m['segments.delete']() });
		if (ok) await run(() => admin.deleteSegment(s.id));
	}
</script>

<Sheet bind:open title={m['segments.title']()} side="right" closeLabel={m['common.close']()}>
	<p class="muted">{m['segments.hint']()}</p>
	{#if segments.length}
		<ul class="list">
			{#each segments as s (s.id)}
				<li>
					<a href="/users?segment={encodeURIComponent(s.id)}" onclick={() => (open = false)}>{s.name}</a>
					<span class="nums muted">{number(s.members)}</span>
					<span class="acts">
						<Button size="sm" variant="secondary" onclick={() => rename(s)}>{m['segments.rename']()}</Button>
						<Button size="sm" variant="secondary" onclick={() => remove(s)}>{m['segments.delete']()}</Button>
					</span>
				</li>
			{/each}
		</ul>
	{/if}
	{#snippet footer()}
		<Button onclick={create}>{m['segments.new']()}</Button>
	{/snippet}
</Sheet>

<style>
	.list {
		display: grid;
		gap: var(--space-05);
		margin: var(--space-1) 0 0;
		padding: 0;
		list-style: none;
	}
	.list li {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1);
	}
	.acts {
		display: inline-flex;
		gap: var(--space-05);
		margin-left: auto;
	}
</style>
