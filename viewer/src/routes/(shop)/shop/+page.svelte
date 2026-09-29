<script lang="ts">
	import ProductItem from '$lib/components/shop/products/product-item.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();
	const catalog = $derived(data.catalog);
	// One tile per variant (colourway / test press), like the original shop: sold-out pieces stay visible
	const tiles = $derived(
		catalog.products.flatMap((product) =>
			product.variants.map((variant) => ({ product, variantId: variant.id }))
		)
	);
</script>

<div class="view shop-view">
	<header class="shop-header">
		<h1>{m['shop.title']()}</h1>
	</header>

	<section>
		<p>
			{m['shop.description']()	}
		</p>
	</section>

	<section>
		<div class="products-grid">
			{#each tiles as tile (tile.product.id + '/' + tile.variantId)}
				<ProductItem product={tile.product} variantId={tile.variantId} currency={catalog.currency} />
			{/each}
		</div>
	</section>
</div>

<style>
	.view.shop-view {
		max-width: 1400px;
		margin: 5rem auto;
	}
	header {
		padding: 0 1rem;
		h1 {
			display: flex;
			align-items: center;
			gap: 0.5rem;
		}
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		gap: 1rem;
	}
	section {
		padding: 1rem;
	}

	.products-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
		gap: 2rem;
	}

	@media (max-width: 768px) {
		.products-grid {
			grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
			gap: 1.5rem;
		}
	}

	@media (max-width: 480px) {
		.products-grid {
			grid-template-columns: 1fr;
			gap: 1rem;
		}
	}
</style>
