<script lang="ts">
	import { Field, Section } from '@scutoons/ui';
	import type { Campaign, NewsletterGiftReach } from '@scutoons/api';
	import { admin } from '$lib/api.ts';
	import { day, number } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A newsletter's gift (#66, agents/research/CAMPAIGN-GIFTS.md §4): a "Verschenken" campaign, and what each group of
	 * the chosen recipients gets from it – members sign in, QR owners sign up with their product code, people who gave
	 * nothing but their email get a personal promo code and with it a new account (owner: "das das in der ui klar wird").
	 * The `{gift}` line of the text becomes the button.
	 */
	let {
		newsletterId,
		value,
		audiences,
		editable,
		onchange
	}: { newsletterId: number; value: string | null; audiences: string[]; editable: boolean; onchange: (next: string | null) => void } = $props();

	let campaigns = $state<Campaign[]>([]);
	let reach = $state<NewsletterGiftReach | null>(null);

	$effect(() => {
		admin
			.campaigns()
			.then((r) => (campaigns = r.items.filter((c) => c.kind === 'giveaway' && (c.status !== 'stopped' || c.id === value))))
			.catch(() => {});
	});

	$effect(() => {
		const chosen = [...audiences];
		if (!value) return;
		admin
			.newsletterGift(newsletterId, chosen)
			.then((r) => (reach = r.reach))
			.catch(() => (reach = null));
	});
</script>

<Section title={m['newsletter.gift']()}>
	<Field label={m['newsletter.gift_campaign']()} hint={m['newsletter.gift_hint']({ line: '{gift}' })}>
		{#snippet children({ id, describedby })}
			<select {id} aria-describedby={describedby} disabled={!editable} value={value ?? ''} onchange={(e) => onchange(e.currentTarget.value || null)}>
				<option value="">{m['newsletter.gift_none']()}</option>
				{#each campaigns as c (c.id)}
					<option value={c.id}>{m['newsletter.gift_option']({ name: c.name, until: day(c.ends_at) })}</option>
				{/each}
			</select>
		{/snippet}
	</Field>
	{#if value && reach}
		<ul class="reach">
			<li>{m['newsletter.gift_members']({ count: number(reach.members) })}</li>
			<li>{m['newsletter.gift_product_code']({ count: number(reach.product_code) })}</li>
			<li>{m['newsletter.gift_email_only']({ count: number(reach.email_only) })}</li>
		</ul>
	{/if}
</Section>

<style>
	.reach {
		margin: var(--space-1) 0 0;
		padding-left: 1.25rem;
		display: grid;
		gap: var(--space-05);
	}
</style>
