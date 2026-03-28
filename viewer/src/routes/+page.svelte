<script lang="ts">
	import ProductGallery from '$lib/components/merch/product/product-gallery.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';

	const { data } = $props();
	let featuredProducts = data.featuredProducts;
	let products = data.products;
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
		2.&Auml;ger<br />
		1. November<br />
		anno felis 2025
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
	<article class="product">
		<h3>Cat Pope Dago Hoodie</h3>
		<div class="product-meta">
			Sizes: M, L, XL, XXL<br />
			Colors: Black, Beige, Grey<br />
			Price: 99.90 EUR<br />
			Limited Edition: 12 Pieces<br />
			<a
			class="btn"
			href={products.hoodie_black.link}
			target="_blank">BUY</a
			>
		</div>
		<!-- Hoodie Galleries -->
		{#each featuredProducts as featuredProduct}
		{#if featuredProduct.key === 'hoodie_black' || featuredProduct.key === 'hoodie_beige' || featuredProduct.key === 'hoodie_grey'}
		<ProductGallery product={products[featuredProduct.key]} label={featuredProduct.label} />
		{/if}
		{/each}
		<article class="product">
			<h3>Cat Pope Dago Heavy T</h3>
			<div class="product-meta">
				Sizes: S, M, L, XL<br />
				Colors: White, Black<br />
				Price: 59.90 EUR<br />
				Limited Edition: 25 Pieces<br />
				<a
				class="btn"
				href={products.tshirt_white.link}
				target="_blank">BUY</a
				><br />
			</div>
			<!-- T-Shirt Galleries -->
			{#each featuredProducts as featuredProduct}
			{#if featuredProduct.key === 'tshirt_white' || featuredProduct.key === 'tshirt_black'}
			<ProductGallery product={products[featuredProduct.key]} label={featuredProduct.label} />
			{/if}
			{/each}
		</article>
		<div class="credits">
			Credits:<br />
			Design: Tatjana Stürmer<br />
			Print: Louise Rath
		</div>
	</article>
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
