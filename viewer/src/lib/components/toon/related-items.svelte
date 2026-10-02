<script lang="ts">
	import { onMount } from 'svelte';
	import { isSoldOut, formatPrice, pickVariant, variantPrice, type ProductCatalog } from '@scutoons/content';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import { loadProducts, productOf } from '$lib/products.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from '$lib/components/items/item-icon.svelte';

	/**
	 * The season's related items on the toon page (`season.related_items`, agents/ITEMS.md "Players, albums, related
	 * items"; owner 2026-10-02: "nice to upsell physical products on the season landing page"): icon + name from the item
	 * catalog, and for a physical item with a shop product its price + "Kaufen" (the product's Bandcamp link, like the
	 * landing page's editions) – or "Ausverkauft". Items the reader already owns say so. Unknown ids are skipped.
	 */
	let { ids }: { ids: string[] } = $props();

	const inventory = getInventoryContext();
	const locale = getLocale();
	let products = $state<ProductCatalog | null>(null);
	onMount(() => {
		loadProducts().then((c) => (products = c));
	});

	const items = $derived(ids.map((id) => inventory.item(id)).filter((item) => !!item));
	const owns = (id: string) => inventory.entries.some((e) => e.item_id === id);
</script>

{#if items.length}
	<section class="related" aria-labelledby="season-related" data-testid="season-related">
		<h3 id="season-related">{m['season.related']()}</h3>
		<ul>
			{#each items as item (item.id)}
				{@const product = item.kind === 'physical' ? productOf(products, item) : undefined}
				{@const name = localized(item.name)}
				<li data-item={item.id}>
					<div class="thumb"><ItemIcon srcs={[item.icon]} {name} /></div>
					<div class="info">
						<span class="name">{name}</span>
						{#if owns(item.id)}
							<span class="meta" data-testid="related-owned">{m['season.related_owned']()}</span>
						{:else if product && !isSoldOut(product)}
							<span class="meta">{formatPrice(variantPrice(product, pickVariant(product)), products?.currency ?? 'EUR', locale)}</span>
						{/if}
					</div>
					{#if product && isSoldOut(product)}
						<span class="meta sold-out">{m['season.sold_out']()}</span>
					{:else if product}
						<a class="btn btn-secondary" href={product.link} target="_blank" rel="noopener noreferrer" data-testid="related-buy">{m['season.buy']()}</a>
					{/if}
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.related {
		padding: var(--space-15) 2rem 0;
	}
	h3 {
		margin: 0 0 0.5rem;
		font-size: 1rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: var(--border-width-regular, 2px) solid var(--ink);
	}
	li {
		display: flex;
		align-items: center;
		gap: var(--space-1, 0.75rem);
		padding: 0.5rem 0;
		border-bottom: var(--border-width-hairline, 1px) solid var(--border-subtle);
	}
	.thumb {
		width: 4rem;
		flex: none;
	}
	.info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.meta {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.btn {
		flex: none;
		display: inline-block;
		text-decoration: none;
	}
	@media (max-width: 768px) {
		.related {
			padding-left: 1rem;
			padding-right: 1rem;
		}
	}
	/* desktop: in the season column (no side padding), sizes on the 24 px root like the music button */
	@media (min-width: 1024px) {
		.related {
			padding: 0.5rem 0 0;
		}
		h3 {
			font-size: 0.6667rem;
			text-transform: uppercase;
			letter-spacing: 0.08em;
		}
		.name {
			font-size: 0.6667rem;
		}
		.meta {
			font-size: 0.5rem;
		}
		.thumb {
			width: 3rem;
		}
		.btn {
			font-size: 0.6667rem;
			padding: 0.5833rem 1.1667rem;
		}
	}
</style>
