<script lang="ts">
	import '@scutoons/tokens/index.css';
	import '../app.css';
	import type { Snippet } from 'svelte';
	import ShopHeader from '$lib/components/shop-header.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { children }: { children: Snippet } = $props();
</script>

<svelte:head>
	<title>{m['shop.title']()}</title>
</svelte:head>

<ShopHeader />

<main class="content">
	{@render children()}
</main>
