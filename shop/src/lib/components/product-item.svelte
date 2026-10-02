<script lang="ts">
	import { asset } from '$lib/cdn.ts';
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
	} from '@scutoons/content';

	/** `variantId` pins the card to one colourway (shop grid: one tile per variant, sold-out ones included) */
	let {
		product,
		currency = 'EUR',
		variantId = null
	}: { product: Product; currency?: string; variantId?: string | null } = $props();

	const locale = getLocale();
	let selectedId = $state<string | null>(null);
	let currentImageIndex = $state(0);

	const pinned = $derived(product.variants.find((v) => v.id === variantId) ?? null);
	const variant = $derived(
		pinned ?? product.variants.find((v) => v.id === selectedId) ?? pickVariant(product)
	);
	const images = $derived(variant.images);
	const hasVariantChoice = $derived(!pinned && product.variants.length > 1);
	const title = $derived(
		pinned && product.variants.length > 1
			? `${product.name} – ${localized(pinned.name, locale)}`
			: product.name
	);
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

<div class="product-item" class:is-sold-out={soldOut || variant.soldOut}>
	<div class="image-container">
		<img src={asset(images[currentImageIndex % images.length])} alt={title} loading="lazy" />
		{#if soldOut || variant.soldOut}
			<span class="sold-out-badge">{m['shop.sold_out']()}</span>
		{/if}
	</div>
	<div class="product-info">
		<h3>{title}</h3>
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

	.is-sold-out .image-container img {
		opacity: 0.6;
	}

	.sold-out-badge {
		position: absolute;
		top: 1rem;
		left: 1rem;
		padding: 0.25rem 0.5rem;
		background: #000;
		color: #fff;
		font-size: 0.75rem;
		text-transform: uppercase;
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
		box-sizing: border-box;
		min-height: var(--control-height);
		padding: 0 0.5rem;
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
