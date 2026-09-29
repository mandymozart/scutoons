<script lang="ts">
	import ProductSlides from '$lib/components/home/product-slides.svelte';
	import {
		editionPieces,
		formatPrice,
		isSoldOut,
		localized,
		pickVariant,
		variantPrice
	} from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { shopUrl } from '$lib/config.ts';

	const { data } = $props();
	const locale = getLocale();
	const currency = $derived(data.catalog?.currency ?? 'EUR');
	// API /products/featured/ -> [{ product, variant }] resolved against the catalog (/products/)
	const featured = $derived(
		data.featured.flatMap((f) => {
			const product = data.catalog?.products.find((p) => p.id === f.product);
			return product ? [{ product, variant: pickVariant(product, f.variant) }] : [];
		})
	);
	const credits = $derived([...new Set(featured.flatMap(({ product }) => product.credits))]);
</script>

<div class="page">
	<div class="logo">
		<img src="SCUTOONS-logo.svg" alt="SCUTOONS" />
		<p>{m.slogan()}</p>
	</div>
	<p>{m.next_release()}</p>
	<section class="releases">
		<p>
			<a href={localizeHref('catpope/')}>Katzenpapst</a><br />
			2.Ärger<br />
			Ank&uuml;ndigung folgt
		</p>
		<p>
			<a href={localizeHref('catpope/')}>Cat Pope</a><br />
			2.Anger<br />
			to be announced later
		</p>

		<p>
			<a href={localizeHref('catpope/')}>キャットポ</a><br />
			2.疾風<br />
			日本語版後日発表
		</p>

		<p>
			<a href={localizeHref('catpope/')}>캣 포프</a><br />
			2.분노<br />
			한국어판 추후 발표
		</p>
	</section>
	<section class="cta">
		SCUTOONS are exclusive limited editions available with the purchase of a physical artefact.
		<p>
			<small>
				Digital download included<br />
				Worldwide shipping available<br />
			</small>
		</p>
		{#each featured as { product, variant } (product.id)}
			<article class="product">
				<h3>{product.name}</h3>
				<div class="product-meta">
					{#if product.sizes.length}
						{m['shop.sizes']()}:
						{#each product.sizes as size, i (size)}{#if product.soldOutSizes.includes(size)}<del
									>{size}</del
								>{:else}{size}{/if}{i < product.sizes.length - 1 ? ', ' : ''}{/each}<br />
					{/if}
					{#if product.variants.length > 1}
						{m['shop.colors']()}:
						{#each product.variants as v, i (v.id)}{#if v.soldOut}<del>{localized(v.name, locale)}</del
								>{:else}{localized(v.name, locale)}{/if}{i < product.variants.length - 1
								? ', '
								: ''}{/each}<br />
					{/if}
					{m['shop.price']()}: {formatPrice(variantPrice(product, variant), currency, locale)}<br />
					{#if editionPieces(product)}
						{m['shop.limited_edition_of']({ edition: editionPieces(product) ?? '' })}<br />
					{/if}
					{#if isSoldOut(product)}
						<span class="btn disabled">{m['shop.sold_out']()}</span>
					{:else}
						<a class="btn" href={product.link} target="_blank" rel="noopener noreferrer"
							>{m['shop.buy']()}</a
						>
					{/if}
				</div>
				<ProductSlides
					images={variant.images}
					label={`${product.name} – ${localized(variant.name, locale)}`}
				/>
			</article>
		{/each}
		{#if featured.length}
			<p class="shop-link">
				<a class="btn btn-secondary" href={shopUrl(locale)}>{m.limited_editions()}</a>
			</p>
		{/if}
		{#if credits.length}
			<div class="credits">
				{m['shop.credits']()}:<br />
				{#each credits as line (line)}{line}<br />{/each}
			</div>
		{/if}
	</section>
	<footer>
		<p>
			© 2025 SCUTOONS - <a href={localizeHref('/attributions')}>{m.attributions()}</a> |
			<a href={localizeHref('/events')}>{m.events()}</a>
		</p>
	</footer>
</div>

<style>
	.page {
		padding: 1rem;
	}
	.releases {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.cta {
		text-align: center;
		font-size: 1.2rem;
		margin: 2rem 0;
	}

	.cta .btn {
		display: inline-block;
		margin-bottom: 2rem;
		padding: 0.5rem 2rem;
		text-decoration: none;
		border: 2px solid;
		transition: opacity 0.2s ease;
	}

	@media (min-width: 768px) {
		.releases {
			grid-template-columns: repeat(3, 1fr);
		}
	}
</style>
