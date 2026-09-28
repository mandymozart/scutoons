<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import {
		editionPieces,
		formatPrice,
		localized,
		pickVariant,
		isSoldOut,
		variantPrice,
		type Product
	} from './shop.ts';

	let { product, currency = 'EUR' }: { product: Product; currency?: string } = $props();

	const locale = getLocale();
	let selectedId = $state<string | null>(null);
	let currentImageIndex = $state(0);

	const variant = $derived(
		product.variants.find((v) => v.id === selectedId) ?? pickVariant(product)
	);
	const images = $derived(variant.images);
	const hasVariantChoice = $derived(product.variants.length > 1);
	const pieces = $derived(editionPieces(product));
	const soldOut = $derived(isSoldOut(product));
	const priceLabel = $derived(formatPrice(variantPrice(product, variant), currency, locale));

	function select(id: string) {
		selectedId = id;
		currentImageIndex = 0;
	}

	$effect(() => {
		const count = images.length;
		if (count < 2) return;
		const timer = setInterval(() => {
			currentImageIndex = (currentImageIndex + 1) % count;
		}, 5000);
		return () => clearInterval(timer);
	});
</script>

<div class="product-item">
	<div class="image-container">
		<img src={asset(images[currentImageIndex % images.length])} alt={product.name} loading="lazy" />
	</div>
	<div class="product-info">
		<h3>{product.name}</h3>
		<p class="price">{priceLabel}</p>
		{#if hasVariantChoice}
			<div class="variants" role="group" aria-label={m['shop.variants']()}>
				{#each product.variants as v (v.id)}
					<button
						type="button"
						class="chip"
						class:active={v.id === variant.id}
						class:sold-out={v.soldOut}
						aria-pressed={v.id === variant.id}
						onclick={() => select(v.id)}
					>
						{#if v.soldOut}<del>{localized(v.name, locale)}</del>{:else}{localized(v.name, locale)}{/if}
					</button>
				{/each}
			</div>
		{/if}
		{#if product.sizes.length}
			<p class="meta">
				{m['shop.sizes']()}:
				{#each product.sizes as size, i (size)}{#if product.soldOutSizes.includes(size)}<del
							>{size}</del
						>{:else}{size}{/if}{i < product.sizes.length - 1 ? ', ' : ''}{/each}
			</p>
		{/if}
		{#if pieces}
			<p class="meta">
				{m['shop.limited_edition_of']({ edition: pieces })}{#if product.remaining}
					· {m['shop.remaining']({ count: product.remaining })}{/if}
			</p>
		{/if}
		{#if soldOut || variant.soldOut}
			<button class="btn disabled" disabled>{m['shop.sold_out']()}</button>
		{:else}
			<a href={product.link} target="_blank" rel="noopener noreferrer" class="btn">{m['shop.buy']()}</a>
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
		aspect-ratio: 4 / 5;
		overflow: hidden;
		flex-shrink: 0;
		background: #fff;
	}

	.image-container img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		display: block;
	}

	.product-info {
		padding: 1rem 0;
		flex-grow: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.product-info h3 {
		margin: 0;
		font-size: 1rem;
	}

	.price {
		margin: 0;
	}

	.meta {
		margin: 0;
		font-size: 0.75rem;
		color: #6b6b6b;
	}

	.variants {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}

	.chip {
		font: inherit;
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		background: #fff;
		color: #000;
		border: 2px solid #ccc;
		border-radius: 0;
		cursor: pointer;
	}

	.chip:hover,
	.chip.active {
		border-color: #000;
	}

	.chip.sold-out {
		color: #6b6b6b;
	}

	.btn {
		margin-top: auto;
		align-self: flex-start;
	}
</style>
