<script lang="ts">
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ReaderStats from '$lib/components/items/reader-stats.svelte';
	import MyClaims from './my-claims.svelte';

	/**
	 * Account page "Sammlung": the inventory and the avatar as links to their panels (owner 2026-10-16, ACCOUNT-PAGE.md
	 * Q1 – the full avatar lives in its panel, not twice), the reader's stats, and the ownership claims.
	 */
	const inventory = getInventoryContext();
	const itemCount = $derived(inventory.entries.reduce((sum, e) => sum + e.qty, 0));
</script>

<p class="line">
	<span>{m['account.inventory']()} · {m['account.items_count']({ count: itemCount })}</span>
	<a class="btn btn-secondary" href={localizeHref('/catpope') + '?inventory=open'}>{m['account.open_inventory']()}</a>
</p>
<p class="line">
	<span class="avatar">{m['account.avatar']()} <ReaderStats compact /></span>
	<a class="btn btn-secondary" href={localizeHref('/catpope') + '?avatar=open'} data-testid="open-avatar">{m['account.open_avatar']()}</a>
</p>
<MyClaims />

<style>
	.line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem 1rem;
		flex-wrap: wrap;
		margin: 0;
	}
	.avatar {
		display: inline-flex;
		align-items: center;
		gap: 1rem;
		flex-wrap: wrap;
	}
	.line .btn {
		padding: 0 1rem;
		text-decoration: none;
	}
</style>
