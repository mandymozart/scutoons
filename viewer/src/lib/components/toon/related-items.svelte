<script lang="ts">
	import { onMount } from 'svelte';
	import { isSoldOut, formatPrice, pickVariant, variantPrice, type ProductCatalog } from '@scutoons/content';
	import { getInventoryContext, localized } from '$lib/contexts/inventory.svelte.ts';
	import type { StockistList } from '@scutoons/api';
	import { loadProducts, productOf } from '$lib/products.ts';
	import { loadStockists, storesFor, storesHref } from '$lib/stockists.ts';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ItemIcon from '$lib/components/items/item-icon.svelte';

	/**
	 * The season's related items on the toon page (`season.related_items`, agents/ITEMS.md "Players, albums, related
	 * items"; owner 2026-10-02: "nice to upsell physical products on the season landing page"): icon + name from the item
	 * catalog, and for a physical item with a shop product its price + "Kaufen" (the product's Bandcamp link, like the
	 * landing page's editions) – or "Ausverkauft". Items the reader already owns say so. Unknown ids are skipped.
	 * Owner 2026-10-02 (screenshot): no heading or rule; the season's music lives in the row of the record that carries
	 * it – its big cut-out as the picture and "Download the music" as the action (an owner sees no "Buy").
	 * Owner 2026-10-02 ("Where to buy"): next to "Buy" a product that a listed store carries offers "Läden vor Ort" /
	 * "Local stores" → /stores?product=<id> (agents/EMBASSY.md "Stores"), also when sold out online.
	 */
	let {
		ids,
		music = null
	}: {
		ids: string[];
		/** the season's music in the row of `item` (the record carrying the album): its cut-out, and the download page
		 * (null = this reader may not download it, e.g. `owners` only) */
		music?: { item: string; image: string | null; href: string | null } | null;
	} = $props();

	const inventory = getInventoryContext();
	const locale = getLocale();
	let products = $state<ProductCatalog | null>(null);
	let stockists = $state<StockistList | null>(null);
	onMount(() => {
		loadProducts().then((c) => (products = c));
		loadStockists().then((l) => (stockists = l));
	});

	const items = $derived(ids.map((id) => inventory.item(id)).filter((item) => !!item));
	const owns = (id: string) => inventory.entries.some((e) => e.item_id === id);
	const musicItem = $derived(music?.item);
</script>

{#if items.length}
	<section class="related" aria-labelledby="season-related" data-testid="season-related">
		<h3 id="season-related" class="sr-only">{m['season.related']()}</h3>
		<ul>
			{#each items as item (item.id)}
				{@const product = item.kind === 'physical' ? productOf(products, item) : undefined}
				{@const name = localized(item.name)}
				{@const withMusic = musicItem === item.id && !!music}
				{@const owned = owns(item.id)}
				<li data-item={item.id} class:with-music={withMusic}>
					<div class="thumb">
						{#if withMusic && music?.image}
							<img src={music.image} alt="" loading="lazy" decoding="async" data-testid="season-download-image" />
						{:else}
							<ItemIcon srcs={[item.icon]} {name} />
						{/if}
					</div>
					<div class="info">
						<span class="name">{name}</span>
						{#if owned}
							<span class="meta" data-testid="related-owned">{m['season.related_owned']()}</span>
						{:else if product && !isSoldOut(product)}
							<span class="meta">{formatPrice(variantPrice(product, pickVariant(product)), products?.currency ?? 'EUR', locale)}</span>
						{/if}
					</div>
					<div class="actions">
						{#if !owned}
							{#if product && isSoldOut(product)}
								<span class="meta sold-out">{m['season.sold_out']()}</span>
							{:else if product}
								<a class="btn btn-secondary" href={product.link} target="_blank" rel="noopener noreferrer" data-testid="related-buy">{m['season.buy']()}</a>
							{/if}
							{#if product && storesFor(stockists, product.id) > 0}
								<a
									class="btn btn-secondary"
									href={localizeHref(storesHref(product.id))}
									title={m['season.local_stores_title']({ count: storesFor(stockists, product.id) })}
									data-testid="related-stores">{m['season.local_stores']()}</a
								>
							{/if}
						{/if}
						{#if withMusic && music?.href}
							<a class="btn btn-secondary" href={music.href} data-testid="season-download">{m['season.download']()}</a>
						{/if}
					</div>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.related {
		padding: var(--space-15) 2rem 0;
	}
	/* the heading stays for screen readers only (owner 2026-10-02: no visible heading or rule) */
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		border: 0;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		gap: var(--space-1, 0.75rem);
		padding: 0.5rem 0;
	}
	/* a thin line only between items (owner 2026-10-02: not under a single one) */
	li + li {
		border-top: var(--border-width-hairline, 1px) solid var(--border-subtle);
	}
	.thumb {
		width: 4rem;
		flex: none;
	}
	/* the record's cut-out, big like the old music block */
	.with-music .thumb {
		width: 8rem;
	}
	.thumb img {
		display: block;
		width: 100%;
		height: auto;
	}
	.actions {
		flex: none;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 0.5rem;
	}
	/* picture + title on one line, the buttons below side by side – the season column is narrow on every screen */
	li {
		flex-wrap: wrap;
	}
	.actions {
		flex: 1 1 100%;
		flex-direction: row;
		/* up to three buttons (Buy, Local stores, Download the music): they wrap instead of running out of the column */
		flex-wrap: wrap;
	}
	.actions > * {
		flex: 1 1 auto;
		text-align: center;
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
		.name {
			font-size: 0.6667rem;
		}
		.meta {
			font-size: 0.5rem;
		}
		.thumb {
			width: 3rem;
		}
		.with-music .thumb {
			width: 6rem;
		}
		.btn {
			font-size: 0.6667rem;
			padding: 0.5833rem 1.1667rem;
		}
	}
</style>
