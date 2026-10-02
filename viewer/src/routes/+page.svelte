<script lang="ts">
	import ProductSlides from '$lib/components/home/product-slides.svelte';
	import {
		editionPieces,
		formatPrice,
		isSoldOut,
		localized,
		pickVariant,
		Status,
		toonTileColor,
		variantPrice,
		type ToonIndex
	} from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { shopUrl } from '$lib/config.ts';
	import { toonImage } from '$lib/utils/cdn.ts';
	import { onMount } from 'svelte';
	import type { StockistList } from '@scutoons/api';
	import { loadStockists, storesFor, storesHref } from '$lib/stockists.ts';

	/**
	 * The landing page (owner 2026-10-01, design "D · Brutal", agents/DESIGN.md "Landing page"): the big logo + slogan,
	 * TOONS (square tiles in the star's colour: published → "Lesen", announced → "bald"), LIMITED EDITIONS (the editions
	 * chosen in the admin app, each with its slider), the footer. Only finished things: no release dates, no announcements
	 * of chapters. Everything comes from the API (+page.ts); the fixed logo + menu button come from the root layout.
	 */
	const { data } = $props();
	const locale = getLocale();

	// published first (each with its newest readable season), then the announced ones; list order within each
	const toons = $derived([
		...data.toons.filter((t: ToonIndex) => t.status === Status.PUBLISHED && !t.announced),
		...data.toons.filter((t: ToonIndex) => t.announced)
	]);
	const name = (toon: ToonIndex) => toon.i18n[locale]?.name ?? toon.i18n[toon.sourceLanguage ?? 'de']?.name ?? toon.id;
	const color = (toon: ToonIndex) => toonTileColor(data.registry, toon.id, toon.tile) ?? 'var(--ink)';

	// "Läden vor Ort" under an edition that a listed store carries (agents/EMBASSY.md "Stores"); loaded after the page
	let stockists = $state<StockistList | null>(null);
	onMount(() => {
		loadStockists().then((l) => (stockists = l));
	});

	const currency = $derived(data.catalog?.currency ?? 'EUR');
	// API /products/featured/ → [{ product, variant }] resolved against the catalog (/products/); unknown ones are skipped
	const featured = $derived(
		data.featured.flatMap((f) => {
			const product = data.catalog?.products.find((p) => p.id === f.product);
			return product ? [{ product, variant: pickVariant(product, f.variant) }] : [];
		})
	);
</script>

<svelte:head>
	<title>SCUTOONS</title>
</svelte:head>

<div class="home" data-testid="landing">
	<section class="masthead">
		<h1 class="wordmark"><img src="/SCUTOONS-logo.svg" alt="SCUTOONS" width="492" height="193" /></h1>
		<p class="slogan">{m.slogan()}</p>
	</section>

	{#if toons.length}
		<section class="toons" aria-labelledby="home-toons">
			<h2 id="home-toons">{m['home.toons']()}</h2>
			<ul class="tiles">
				{#each toons as toon (toon.id)}
					{@const season = data.seasons[toon.id]}
					<li class="tile-item" class:published={!toon.announced}>
						{#if toon.announced}
							<div class="tile soon" data-testid="toon-tile" data-toon={toon.id} data-state="soon">
								<span class="tile-image" style:background-color={color(toon)}>
									<img src={toonImage(toon)} alt="" loading="lazy" />
								</span>
								<span class="tile-body">
									<span class="tile-text">
										<span class="tile-name">{name(toon)}</span>
										<span class="tile-meta">{m['home.soon']()}</span>
									</span>
								</span>
							</div>
						{:else}
							<a class="tile" href={localizeHref(`/${toon.id}`)} data-testid="toon-tile" data-toon={toon.id} data-state="published">
								<span class="tile-image" style:background-color={color(toon)}>
									<img src={toonImage(toon)} alt="" loading="lazy" />
								</span>
								<span class="tile-body">
									<span class="tile-text">
										<span class="tile-name">{name(toon)}</span>
										{#if season}<span class="tile-meta">{m['home.season']({ number: season.number, title: season.title })}</span>{/if}
									</span>
									<span class="read">
										{m['home.read']()}
										<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M1 8h13M9 3l5 5-5 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="square" /></svg>
									</span>
								</span>
							</a>
						{/if}
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	<section class="editions" aria-labelledby="home-editions">
		<h2 id="home-editions">{m['home.editions']()}</h2>
		<p class="intro">{m['home.editions_intro']()}</p>
		<p class="note">{m['home.editions_note']()}</p>

		{#each featured as { product, variant } (`${product.id}/${variant.id}`)}
			<article class="product" data-testid="edition" data-product={product.id} data-variant={variant.id}>
				<div class="product-info">
					<h3>{product.name}</h3>
					{#if product.sizes.length}
						<span class="line">
							{m['home.sizes']()}:
							{#each product.sizes as size, i (size)}{#if product.soldOutSizes.includes(size)}<del title={m['home.sold_out']()}
										>{size}<span class="visually-hidden"> ({m['home.sold_out']()})</span></del
									>{:else}{size}{/if}{i < product.sizes.length - 1 ? ', ' : ''}{/each}
						</span>
					{/if}
					{#if product.variants.length > 1}
						<span class="line">
							{m['home.colors']()}:
							{#each product.variants as v, i (v.id)}{#if v.soldOut}<del title={m['home.sold_out']()}
										>{localized(v.name, locale)}<span class="visually-hidden"> ({m['home.sold_out']()})</span></del
									>{:else}{localized(v.name, locale)}{/if}{i < product.variants.length - 1 ? ', ' : ''}{/each}
						</span>
					{/if}
					<span class="line price">{formatPrice(variantPrice(product, variant), currency, locale)}</span>
					{#if editionPieces(product)}
						<span class="line meta">{m['home.limited']({ count: editionPieces(product) ?? '' })}</span>
					{/if}
					{#if product.credits.length}
						<span class="line meta credits">{product.credits.join(' · ')}</span>
					{/if}
					{#if isSoldOut(product)}
						<span class="buy disabled" aria-disabled="true">{m['home.sold_out_all']()}</span>
					{:else}
						<a class="buy" href={product.link} target="_blank" rel="noopener noreferrer">{m['home.buy']()}</a>
					{/if}
					{#if storesFor(stockists, product.id) > 0}
						<a class="local" href={localizeHref(storesHref(product.id))} data-testid="edition-stores">{m['home.local_stores']()}</a>
					{/if}
				</div>
				<div class="product-slides">
					<ProductSlides images={variant.images} label={`${product.name} – ${localized(variant.name, locale)}`} />
				</div>
			</article>
		{/each}
		<a class="all" href={shopUrl(locale)}>{m['home.all_editions']()}</a>
	</section>

	<footer class="footer">
		<span class="claim">{m['home.footer']()}</span>
		<nav aria-label="SCUTOONS">
			{#if stockists?.stores.length}
				<a href={localizeHref('/stores')}>{m['home.stores']()}</a>
				·
			{/if}
			<a href={localizeHref('/events')}>{m['home.events']()}</a>
			·
			<a href={localizeHref('/attributions')}>{m['home.attributions']()}</a>
		</nav>
	</footer>
</div>

<style>
	/* design "D · Brutal": ink on paper, 2px ink rules, square, IBM Plex Mono; sizes in rem (root 14 / 16 / 24 px) */
	.home {
		--home-pad: 1.1429rem; /* 16 px on a phone */
		color: var(--ink);
		/* clear of the fixed logo (top left, 3rem) and the menu button */
		padding-top: 4rem;
		line-height: 1.4;
	}
	h2 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 700;
		line-height: 1.2;
	}
	section {
		padding-left: var(--home-pad);
		padding-right: var(--home-pad);
	}

	.masthead {
		padding-top: 0.5rem;
		padding-bottom: 1.7143rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.wordmark {
		margin: 0;
		line-height: 0;
	}
	.wordmark img {
		display: block;
		width: 100%;
		height: auto;
	}
	.slogan {
		margin: 1rem 0 0;
		font-weight: 700;
	}

	.toons {
		padding-top: 2.8571rem;
	}
	.toons h2 {
		margin-bottom: 1.1429rem;
	}
	.tiles {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 0.8571rem;
	}
	/* phone: a published toon spans the row (name + meta left, "Lesen" right), the announced ones two by two */
	.tile-item.published {
		grid-column: 1 / -1;
	}
	.tile {
		display: flex;
		flex-direction: column;
		height: 100%;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		color: var(--ink);
		text-decoration: none;
	}
	.tile-image {
		display: block;
		aspect-ratio: 1 / 1;
		border-bottom: var(--border-width-regular) solid var(--ink);
		overflow: hidden;
	}
	.tile-image img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.tile-body {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.75rem;
		padding: 0.7143rem;
	}
	.tile-item.published .tile-body {
		padding: 1rem;
	}
	.tile-text {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}
	.tile-name {
		font-weight: 700;
	}
	.tile-meta,
	.note,
	.meta {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.read {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 0.7143rem;
		padding: 1rem 1.4286rem;
		background: var(--ink);
		color: var(--paper);
		line-height: 1;
		white-space: nowrap;
	}
	.read svg {
		width: 1rem;
		height: 1rem;
	}
	.tile:hover .read,
	.tile:focus-visible .read {
		background: var(--border-hover);
	}
	.tile:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 3px;
	}

	.editions {
		padding-top: 4rem;
	}
	.editions h2 {
		margin-bottom: 0.7143rem;
	}
	.intro {
		margin: 0;
		max-width: 42rem;
	}
	.note {
		margin: 0.4286rem 0 0;
	}
	.product {
		margin-top: 2.8571rem;
		padding-top: 1.4286rem;
		border-top: var(--border-width-regular) solid var(--ink);
		display: flex;
		flex-direction: column;
		gap: 0.8571rem;
	}
	.product-info {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.4286rem;
		text-align: center;
	}
	.product h3 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 700;
		line-height: 1.2;
	}
	.price {
		font-weight: 700;
	}
	del {
		color: var(--text-muted);
	}
	.buy {
		margin-top: 0.7143rem;
		padding: 1rem 3rem;
		background: var(--ink);
		color: var(--paper);
		text-decoration: none;
		line-height: 1;
	}
	.buy:hover {
		background: var(--border-hover);
	}
	.local {
		padding: 1rem 3rem;
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--ink);
		text-decoration: none;
		line-height: 1;
	}
	.local:hover {
		border-color: var(--ink);
	}
	.buy.disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.product-slides {
		min-width: 0;
	}
	.all {
		display: block;
		margin-top: 2.8571rem;
		padding: 1rem;
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--ink);
		text-decoration: none;
		text-align: center;
		line-height: 1;
	}
	.all:hover {
		border-color: var(--ink);
	}

	.footer {
		margin-top: 4rem;
		padding: 1.4286rem var(--home-pad);
		border-top: var(--border-width-regular) solid var(--ink);
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: var(--type-caption-size);
		line-height: 1.6;
	}
	.claim {
		font-weight: 700;
	}
	.footer a {
		color: var(--ink);
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}

	/* tablet + desktop (root 24 px): every tile the same, three across */
	@media (min-width: 768px) {
		.home {
			--home-pad: 2rem; /* 48 px */
		}
		.masthead {
			padding-bottom: 2rem;
		}
		.toons {
			padding-top: 3rem;
		}
		.toons h2 {
			margin-bottom: 1.3333rem;
		}
		.tiles {
			grid-template-columns: repeat(3, minmax(0, 1fr));
			gap: 1.3333rem;
		}
		.tile-item.published {
			grid-column: auto;
		}
		.tile-body,
		.tile-item.published .tile-body {
			flex-direction: column;
			align-items: flex-start;
			justify-content: flex-start;
			gap: 0.5rem;
			padding: 0.8333rem;
		}
		.read {
			margin-top: 0.5rem;
			gap: 0.6667rem;
			padding: 0.6667rem 1.3333rem;
		}
		.read svg {
			width: 0.9167rem;
			height: 0.9167rem;
		}
		.editions {
			padding-top: 4rem;
		}
		.product {
			margin-top: 3rem;
		}
		.all {
			display: inline-block;
			padding: 0.9167rem 2.9167rem;
		}
		.footer {
			flex-direction: row;
			justify-content: space-between;
			padding: 1.3333rem var(--home-pad);
			margin-top: 5rem;
		}
	}

	/* desktop: the edition's text left (420 px), its 3-up slider right */
	@media (min-width: 1024px) {
		.product {
			display: grid;
			grid-template-columns: 17.5rem minmax(0, 1fr);
			gap: 2rem;
			padding-top: 1.3333rem;
		}
		.product-info {
			align-items: flex-start;
			text-align: left;
			gap: 0.3333rem;
		}
		.buy {
			margin-top: 0.6667rem;
			padding: 1rem 3rem;
		}
	}
</style>
