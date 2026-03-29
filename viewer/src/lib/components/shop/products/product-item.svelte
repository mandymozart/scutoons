<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { onMount } from 'svelte';

	let { productKey, product } = $props();
	let currentImageIndex = $state(0);

	const displayName = $derived(
		productKey
			.replace(/_/g, ' ')
			.split(' ')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ')
	);

	let autoPlayInterval: ReturnType<typeof setInterval>;

	onMount(() => {
		autoPlayInterval = setInterval(() => {
			currentImageIndex = (currentImageIndex + 1) % product.images.length;
		}, 5000);

		return () => clearInterval(autoPlayInterval);
	});
</script>

<div class="product-item">
	<div class="image-container">
		<img src={asset(product.images[currentImageIndex])} alt={displayName} />
	</div>
	<div class="product-info">
		<h3>{displayName}</h3>
		<p class="price">{product.price}</p>
		<p class="edition">{m['shop.limited_edition_of']({ edition: product.edition })}</p>
		{#if product.soldOut}
			<button class="btn disabled" disabled>{m['shop.sold_out']()}</button>
		{:else}
			<a href={product.link} target="_blank" rel="noopener noreferrer" class="btn"
				>{m['shop.buy']()}</a
			>
		{/if}
	</div>
</div>

<style>
	.product-item {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.image-container {
		position: relative;
		width: 100%;
		overflow: hidden;
		flex-shrink: 0;
	}

	.image-container img {
		width: 100%;
		height: auto;
		object-fit: contain;
		display: block;
	}

	.product-info {
		padding: 1rem 0;
		flex-grow: 1;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
	}

	.product-info h3 {
		margin: 0;
		font-size: 1rem;
	}

	.price {
		margin: 0.5rem 0 0 0;
	}

	.edition {
		margin: 0.25rem 0 .5rem 0;
		font-size: 0.75rem;
	}
</style>
