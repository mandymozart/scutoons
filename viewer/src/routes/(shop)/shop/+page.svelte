<script lang="ts">
	import ProductCard from '$lib/components/shop/product/product-card.svelte';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();
	let products = data.products;
</script>

<div class="shop-page">
	<div class="shop-header">
		<h1>SCUTOONS Shop</h1>
		<p>
			Explore our exclusive limited edition merchandise featuring your favorite SCUTOONS characters.
			Each item is crafted with care and designed to bring a smile to your face. Browse through our
			collection and find the perfect addition to your SCUTOONS memorabilia!
		</p>
	</div>

	<div class="products-grid">
		{#each Object.entries(products) as [productKey, product]}
			<ProductCard {productKey} {product} />
		{/each}
	</div>
</div>

<style>
	.shop-page {
		padding: 2rem 1rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.shop-header {
		text-align: center;
		margin-bottom: 3rem;
	}

	.shop-header h1 {
		font-size: 2.5rem;
		margin: 0 0 1rem 0;
	}

	.shop-header p {
		font-size: 1.1rem;
		max-width: 600px;
		margin: 0 auto;
		color: #666;
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

		.shop-header h1 {
			font-size: 2rem;
		}
	}

	@media (max-width: 480px) {
		.shop-page {
			padding: 1rem;
		}

		.products-grid {
			grid-template-columns: 1fr;
			gap: 1rem;
		}

		.shop-header h1 {
			font-size: 1.5rem;
		}
	}
</style>
