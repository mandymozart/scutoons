<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * Transparent cut-out icon of an item. Falls back through `srcs` and finally to an ink-line
	 * placeholder with the item's initials (cut-outs may not exist on the CDN yet).
	 */
	let { srcs, name, size = '100%' }: { srcs: string[]; name: string; size?: string } = $props();

	let failed = $state(0);
	const candidates = $derived([...new Set(srcs.filter(Boolean))]);
	const src = $derived(candidates[failed] ? asset(candidates[failed]) : '');
	const initials = $derived(
		name
			.split(/[\s_·-]+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((w) => w[0]?.toUpperCase())
			.join('')
	);

	// new item → start over (string key: re-renders with equal sources don't retry)
	const key = $derived(candidates.join('|'));
	$effect(() => {
		key;
		failed = 0;
	});
</script>

<div class="item-icon" style="width:{size}">
	{#if src}
		<img {src} alt={name} title={name} draggable="false" onerror={() => (failed += 1)} />
	{:else}
		<div class="placeholder" title={name} aria-label={name}>{initials}</div>
	{/if}
</div>

<style>
	.item-icon {
		aspect-ratio: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		user-select: none;
	}
	.placeholder {
		width: 80%;
		height: 80%;
		border: 2px solid currentColor;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 1rem;
		letter-spacing: 0.05em;
	}
</style>
