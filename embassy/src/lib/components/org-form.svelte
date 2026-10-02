<script lang="ts">
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { ORG_TYPES, TIERS, isStockist, type Org, type OrgInput } from '@scutoons/api';
	import type { Product } from '@scutoons/content';
	import { label, splitList } from '$lib/labels.ts';
	import { loadProducts, productIcon } from '$lib/products.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The organisation's fields; `onsubmit` gets an OrgInput (lists split, country upper case) */
	let {
		org = null,
		onsubmit,
		actions
	}: { org?: Org | null; onsubmit: (input: OrgInput) => void; actions: Snippet } = $props();

	// svelte-ignore state_referenced_locally
	const start = org;
	let name = $state(start?.name ?? '');
	let type = $state<string>(start?.type ?? 'venue');
	let city = $state(start?.city ?? '');
	let country = $state(start?.country ?? '');
	let website = $state(start?.website ?? '');
	let email = $state(start?.email ?? '');
	let phone = $state(start?.phone ?? '');
	let tier = $state<string>(start?.tier ?? '');
	let tags = $state((start?.tags ?? []).join(', '));
	let territories = $state((start?.territories ?? []).join(', '));
	let links = $state((start?.links ?? []).join('\n'));
	let source = $state(start?.source ?? '');
	let notes = $state(start?.notes ?? '');
	let doNotContact = $state(start?.do_not_contact ?? false);
	// stores (agents/EMBASSY.md "Stores"): the public "Where to buy" fields, only for store / record shop
	let listed = $state(start?.listed ?? false);
	let independent = $state(start?.independent ?? false);
	let address = $state(start?.address ?? '');
	let opening = $state(start?.opening ?? '');
	let carries = $state<string[]>([...(start?.carries ?? [])]);
	let products = $state<Product[]>([]);
	onMount(() => {
		loadProducts().then((list) => (products = list));
	});
	const stockist = $derived(isStockist(type));
	// ids the store carries that the catalog doesn't know (any more) stay visible, so they can be removed
	const unknownCarries = $derived(products.length ? carries.filter((id) => !products.some((p) => p.id === id)) : []);

	function toggleCarry(id: string, on: boolean) {
		carries = on ? [...carries.filter((c) => c !== id), id] : carries.filter((c) => c !== id);
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		onsubmit({
			name: name.trim(),
			type: type as Org['type'],
			city: city.trim(),
			country: country.trim().toUpperCase(),
			website: website.trim(),
			email: email.trim(),
			phone: phone.trim(),
			tier: (tier || null) as Org['tier'],
			tags: splitList(tags),
			territories: splitList(territories).map((t) => t.toUpperCase()),
			links: links.split('\n').map((l) => l.trim()).filter(Boolean),
			source: source.trim(),
			notes: notes.trim(),
			do_not_contact: doNotContact,
			// the store fields are kept as they are when the type is not a store (nothing public without the type)
			...(stockist
				? { listed, independent, address: address.trim(), opening: opening.trim(), carries: carries.filter((id) => !unknownCarries.includes(id)) }
				: {})
		});
	}
</script>

<form class="form-grid" onsubmit={submit}>
	<label class="field wide">
		<span>{m['org.name']()}</span>
		<input bind:value={name} required maxlength="255" />
	</label>
	<label class="field">
		<span>{m['org.type']()}</span>
		<select bind:value={type}>
			{#each ORG_TYPES as t (t)}<option value={t}>{label('type', t)}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['org.tier']()}</span>
		<select bind:value={tier}>
			<option value="">{m['org.tier_none']()}</option>
			{#each TIERS as t (t)}<option value={t}>{t}</option>{/each}
		</select>
	</label>
	<label class="field">
		<span>{m['org.city']()}</span>
		<input bind:value={city} />
	</label>
	<label class="field">
		<span>{m['org.country']()}</span>
		<input bind:value={country} maxlength="2" pattern="[A-Za-z]{'{'}2{'}'}" />
	</label>
	<label class="field">
		<span>{m['org.phone']()}</span>
		<input type="tel" bind:value={phone} />
	</label>
	<label class="field">
		<span>{m['org.email']()}</span>
		<input type="email" bind:value={email} />
	</label>
	<label class="field wide">
		<span>{m['org.website']()}</span>
		<input bind:value={website} inputmode="url" />
	</label>
	<label class="field wide">
		<span>{m['org.links']()}</span>
		<textarea rows="2" bind:value={links}></textarea>
	</label>
	<label class="field">
		<span>{m['org.tags']()}</span>
		<input bind:value={tags} />
	</label>
	<label class="field">
		<span>{m['org.territories']()}</span>
		<input bind:value={territories} placeholder="DE, AT, CH" />
	</label>
	<label class="field">
		<span>{m['org.source']()}</span>
		<input bind:value={source} />
	</label>
	<label class="field wide">
		<span>{m['org.notes']()}</span>
		<textarea rows="3" bind:value={notes}></textarea>
	</label>
	<label class="check wide">
		<input type="checkbox" bind:checked={doNotContact} />
		{m['org.do_not_contact']()}
	</label>
	{#if stockist}
		<fieldset class="wide store" data-testid="store-fields">
			<legend>{m['store.title']()}</legend>
			<p class="caption muted">{m['store.intro']()}</p>
			<label class="check" title={m['store.listed_hint']()}>
				<input type="checkbox" bind:checked={listed} data-testid="store-listed" />
				{m['store.listed']()}
			</label>
			<label class="check" title={m['store.independent_hint']()}>
				<input type="checkbox" bind:checked={independent} data-testid="store-independent" />
				{m['store.independent']()}
			</label>
			<div class="pair">
				<label class="field">
					<span>{m['store.address']()}</span>
					<input bind:value={address} maxlength="255" placeholder={m['store.address_placeholder']()} />
				</label>
				<label class="field">
					<span>{m['store.opening']()}</span>
					<input bind:value={opening} maxlength="255" placeholder={m['store.opening_placeholder']()} />
				</label>
			</div>
			<div class="carries" role="group" aria-label={m['store.carries']()}>
				<span class="caption muted">{m['store.carries']()}</span>
				{#if !products.length}
					<p class="caption muted">{m['shell.loading']()}</p>
				{/if}
				<ul>
					{#each products as product (product.id)}
						{@const icon = productIcon(product)}
						<li>
							<label class="check product" title={product.id}>
								<input
									type="checkbox"
									checked={carries.includes(product.id)}
									onchange={(e) => toggleCarry(product.id, e.currentTarget.checked)}
									data-product={product.id}
								/>
								{#if icon}<img src={icon} alt="" width="40" height="40" loading="lazy" />{/if}
								<span>{product.name}</span>
							</label>
						</li>
					{/each}
					{#each unknownCarries as id (id)}
						<li class="caption">{m['store.unknown_product']({ id })}</li>
					{/each}
				</ul>
			</div>
		</fieldset>
	{/if}
	<div class="wide row">{@render actions()}</div>
</form>

<style>
	.store {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		margin: 0;
	}
	.store p {
		margin: 0;
	}
	.pair {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(min(14rem, 100%), 1fr));
		gap: var(--space-075) var(--space-1);
	}
	.carries ul {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(18rem, 100%), 1fr));
		gap: var(--space-05) var(--space-1);
		margin: var(--space-05) 0 0;
		padding: 0;
		list-style: none;
	}
	.product img {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: contain;
		flex: none;
	}
</style>
