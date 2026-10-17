<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { Stockist, StockistList } from '@scutoons/api';
	import type { ProductCatalog } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { shopUrl } from '$lib/config.ts';
	import { loadProducts } from '$lib/products.ts';
	import { loadStockists } from '$lib/stockists.ts';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * "Where to buy" (owner 2026-10-02: "a public page which lists which stores carry which items, so people can buy from a
	 * local supplier of their choice. I want to encourage small vendors at first."): the Embassy's listed stores
	 * (`/stockists/`, agents/EMBASSY.md "Stores"), small / independent shops first, then the other stores; each block
	 * grouped by country and city. Filters `?product=` (from a product's "Local stores" link) and `?country=` live in the
	 * URL. Texts from the messages, product names + icons from the shop catalog, country names from the browser (Intl).
	 */
	const locale = getLocale();
	let list = $state<StockistList | null>(null);
	let catalog = $state<ProductCatalog | null>(null);
	let loaded = $state(false);

	onMount(() => {
		Promise.all([loadStockists(), loadProducts()]).then(([stockists, products]) => {
			list = stockists;
			catalog = products;
			loaded = true;
		});
	});

	const product = $derived(page.url.searchParams.get('product') ?? '');
	const country = $derived((page.url.searchParams.get('country') ?? '').toUpperCase());

	const regionNames = (() => {
		try {
			return new Intl.DisplayNames([locale], { type: 'region' });
		} catch {
			return null;
		}
	})();
	const countryName = (code: string) => (code ? (regionNames?.of(code) ?? code) : m['stores.country_unknown']());
	const productOf = (id: string) => catalog?.products.find((p) => p.id === id);
	const productName = (id: string) => productOf(id)?.name ?? id;

	// filter options: the products some store carries (catalog order), plus a product asked for in the URL
	const productOptions = $derived.by(() => {
		const carried = Object.keys(list?.products ?? {});
		const ids = catalog ? catalog.products.map((p) => p.id).filter((id) => carried.includes(id)) : carried;
		return product && !ids.includes(product) ? [...ids, product] : ids;
	});
	const countryOptions = $derived(
		[...(list?.countries ?? []), ...(country && !list?.countries.includes(country) ? [country] : [])].sort((a, b) =>
			countryName(a).localeCompare(countryName(b), locale)
		)
	);

	const stores = $derived(
		(list?.stores ?? []).filter((s) => (!product || s.carries.includes(product)) && (!country || s.country === country))
	);
	const independent = $derived(stores.filter((s) => s.independent));
	const others = $derived(stores.filter((s) => !s.independent));

	type CityGroup = { city: string; stores: Stockist[] };
	type CountryGroup = { country: string; name: string; cities: CityGroup[] };
	/** country (by its name in the reader's language) → city → stores (the API's order inside a city) */
	function grouped(items: Stockist[]): CountryGroup[] {
		const countries = new Map<string, Map<string, Stockist[]>>();
		for (const store of items) {
			const cities = countries.get(store.country) ?? new Map<string, Stockist[]>();
			countries.set(store.country, cities);
			cities.set(store.city, [...(cities.get(store.city) ?? []), store]);
		}
		return [...countries.entries()]
			.map(([code, cities]) => ({
				country: code,
				name: countryName(code),
				cities: [...cities.entries()]
					.map(([city, list]) => ({ city, stores: list }))
					.sort((a, b) => a.city.localeCompare(b.city, locale))
			}))
			.sort((a, b) => (!a.country ? 1 : !b.country ? -1 : a.name.localeCompare(b.name, locale)));
	}

	function setFilter(key: 'product' | 'country', value: string) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		goto(url.pathname + url.search, { replaceState: true, noScroll: true, keepFocus: true });
	}

	const typeLabel = (store: Stockist) => (store.type === 'record_shop' ? m['stores.type_record_shop']() : m['stores.type_store']());
	const host = (url: string) => url.replace(/^https?:\/\/(www\.)?/, '').replace(/\/$/, '');
	const asked = $derived(product ? productOf(product) : undefined);
</script>

<svelte:head>
	<title>{m['stores.title']()} · SCUTOONS</title>
</svelte:head>

{#snippet storeList(items: Stockist[])}
	{#each grouped(items) as group (group.country)}
		<section class="country" aria-label={group.name} data-country={group.country}>
			<h3>{group.name}</h3>
			{#each group.cities as city (city.city)}
				<h4 class="city">{city.city || group.name}</h4>
				<ul class="stores">
					{#each city.stores as store (store.id)}
						<li class="store" data-testid="store" data-store={store.id} data-independent={store.independent}>
							<span class="name">{store.name}</span>
							<span class="meta">
								{typeLabel(store)}{#if store.independent}<span class="tag">{m['stores.independent_tag']()}</span>{/if}
							</span>
							{#if store.address}<span class="line">{store.address}</span>{/if}
							{#if store.opening}<span class="line meta">{store.opening}</span>{/if}
							<ul class="carries" aria-label={m['stores.carries']()}>
								{#each store.carries as id (id)}
									{@const icon = productOf(id)?.icon}
									<li class:current={id === product}>
										{#if icon}<img src={asset(icon)} alt="" loading="lazy" decoding="async" />{/if}
										<span>{productName(id)}</span>
									</li>
								{/each}
							</ul>
							{#if store.website}
								<a class="visit" href={store.website} target="_blank" rel="noopener noreferrer" data-testid="store-link">
									{host(store.website)}
									<Icon name="chevron-right" size="1rem" />
								</a>
							{/if}
						</li>
					{/each}
				</ul>
			{/each}
		</section>
	{/each}
{/snippet}

<div class="where" data-testid="stores-page">
	<header class="head">
		<h1>{m['stores.title']()}</h1>
		<p class="intro">{m['stores.intro']()}</p>
	</header>

	{#if !loaded}
		<p class="state">{m['common.loading']()}</p>
	{:else if !list?.stores.length}
		<section class="empty" data-testid="stores-empty">
			<p>{m['stores.empty']()}</p>
			<a class="all" href={shopUrl(locale)}>{m['stores.buy_online']()}</a>
		</section>
	{:else}
		<form class="filters" onsubmit={(e) => e.preventDefault()}>
			<label>
				<span>{m['stores.filter_product']()}</span>
				<select value={product} onchange={(e) => setFilter('product', e.currentTarget.value)} data-testid="filter-product">
					<option value="" selected={!product}>{m['stores.all_products']()}</option>
					{#each productOptions as id (id)}<option value={id} selected={id === product}>{productName(id)}</option>{/each}
				</select>
			</label>
			<label>
				<span>{m['stores.filter_country']()}</span>
				<select value={country} onchange={(e) => setFilter('country', e.currentTarget.value)} data-testid="filter-country">
					<option value="" selected={!country}>{m['stores.all_countries']()}</option>
					{#each countryOptions as code (code)}<option value={code} selected={code === country}>{countryName(code)}</option>{/each}
				</select>
			</label>
		</form>

		{#if !stores.length}
			<section class="empty" data-testid="stores-empty">
				<p>{product ? m['stores.empty_product']({ product: productName(product) }) : m['stores.empty_filter']()}</p>
				{#if asked?.link}
					<a class="all" href={asked.link} target="_blank" rel="noopener noreferrer">{m['stores.buy_online']()}</a>
				{/if}
				<a class="reset" href={localizeHref('/stores')}>{m['stores.show_all']()}</a>
			</section>
		{:else}
			{#if independent.length}
				<section class="block" aria-labelledby="stores-independent" data-testid="stores-independent">
					<h2 id="stores-independent">{m['stores.independent']()}</h2>
					<p class="note">{m['stores.independent_note']()}</p>
					{@render storeList(independent)}
				</section>
			{/if}
			{#if others.length}
				<section class="block" aria-labelledby="stores-others" data-testid="stores-others">
					<h2 id="stores-others">{independent.length ? m['stores.others']() : m['stores.all_stores']()}</h2>
					{@render storeList(others)}
				</section>
			{/if}
		{/if}
	{/if}
</div>

<style>
	/* the landing page's design "D · Brutal": ink on paper, 2px ink rules, square, IBM Plex Mono; rem on the fluid root */
	.where {
		--pad: 1.1429rem; /* 16 px on a phone */
		color: var(--ink);
		/* clear of the fixed logo (top left, 3rem) and the menu button */
		padding: 4rem var(--pad) 4rem;
		line-height: 1.4;
		max-width: 60rem;
		box-sizing: border-box;
	}
	h1,
	h2 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 700;
		line-height: 1.2;
		text-transform: uppercase;
	}
	.head {
		padding: 0.5rem 0 1.7143rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.intro {
		margin: 0.7143rem 0 0;
		max-width: 42rem;
	}
	.state {
		margin-top: 2rem;
	}
	.note,
	.meta {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}

	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 0.8571rem 1.4286rem;
		margin-top: 1.7143rem;
	}
	.filters label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		flex: 1 1 14rem;
		min-width: 0;
	}
	.filters span {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	select {
		width: 100%;
		box-sizing: border-box;
		height: var(--control-height);
		padding: 0 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	select:focus-visible,
	a:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}

	.block {
		margin-top: 2.8571rem;
	}
	.block h2 {
		margin-bottom: 0.4286rem;
	}
	.note {
		margin: 0;
	}
	.country {
		margin-top: 1.7143rem;
		padding-top: 1rem;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 700;
		line-height: 1.2;
	}
	.city {
		margin: 1rem 0 0;
		font-size: var(--type-caption-size);
		font-weight: 400;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--text-muted);
	}
	.stores {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 0.8571rem;
		margin-top: 0.5rem;
	}
	.store {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 1rem;
		border: var(--border-width-regular) solid var(--ink);
	}
	.name {
		font-weight: 700;
	}
	.tag {
		margin-left: 0.5rem;
		padding: 0.1rem 0.4rem;
		background: var(--ink);
		color: var(--paper);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.carries {
		list-style: none;
		margin: 0.5rem 0 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 1rem;
	}
	.carries li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.carries li.current {
		text-decoration: underline;
		text-decoration-thickness: 2px;
		text-underline-offset: 3px;
	}
	.carries img {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: contain;
		flex: none;
	}
	.visit {
		align-self: flex-start;
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.7143rem;
		padding: 0.7143rem 1rem;
		background: var(--ink);
		color: var(--paper);
		text-decoration: none;
		line-height: 1;
		max-width: 100%;
		overflow-wrap: anywhere;
	}
	.visit:hover {
		background: var(--border-hover);
	}

	.empty {
		margin-top: 2rem;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 1rem;
	}
	.empty p {
		margin: 0;
		max-width: 42rem;
	}
	.all {
		display: inline-block;
		padding: 1rem 1.5rem;
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--ink);
		text-decoration: none;
		line-height: 1;
	}
	.all:hover {
		border-color: var(--ink);
	}
	.reset {
		color: var(--ink);
	}

	/* tablet + desktop (root 24 px): two stores side by side */
	@media (min-width: 768px) {
		.where {
			--pad: 2rem;
			margin: 0 auto;
		}
		.stores {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: 1rem;
		}
		.carries img {
			width: 1.75rem;
			height: 1.75rem;
		}
		.visit {
			padding: 0.5rem 0.8333rem;
		}
	}
</style>
