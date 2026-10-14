<script lang="ts">
	// Draft placeholders for the cave's furniture and the media without icons yet (the owner's art comes later);
	// items with an icon on the CDN use it.
	import { asset } from '$lib/utils/cdn.ts';

	let { kind, count = 0 }: { kind: string; count?: number } = $props();
	const ICONS: Record<string, string> = {
		awakening_vinyl: 'catpope/items/awakening_vinyl.webp',
		awakening_test_press: 'catpope/items/awakening_test_press.webp'
	};
</script>

{#if ICONS[kind]}
	<img src={asset(ICONS[kind])} alt="" />
{:else}
	<svg viewBox="0 0 100 100" aria-hidden="true">
		{#if kind === 'cd'}
			<circle cx="50" cy="50" r="40" /><circle cx="50" cy="50" r="8" />
		{:else if kind === 'tape'}
			<rect x="10" y="25" width="80" height="50" /><circle cx="35" cy="50" r="9" /><circle
				cx="65"
				cy="50"
				r="9"
			/>
		{:else if kind === 'songbook_diy' || kind === 'songbook_diy_broken'}
			<rect x="22" y="12" width="56" height="76" />
			{#each [24, 40, 56, 72] as y (y)}<line x1="16" y1={y} x2="28" y2={y} />{/each}
			{#if kind === 'songbook_diy_broken'}<path d="M50 12 L44 40 L56 55 L46 88" />{/if}
		{:else if kind === 'cd_rack' || kind === 'cassette_rack' || kind === 'record_shelf'}
			{@const shelves = kind === 'record_shelf' ? 2 : 4}
			{@const filled = Math.min(count, 12)}
			<rect x="15" y="8" width="70" height="84" />
			{#each Array(shelves) as _, i (i)}<line
					x1="15"
					y1={8 + ((i + 1) * 84) / (shelves + 1)}
					x2="85"
					y2={8 + ((i + 1) * 84) / (shelves + 1)}
				/>{/each}
			{#each Array(filled) as _, i (i)}
				<rect
					class="fill"
					x={19 + (i % 6) * 11}
					y={8 + Math.floor(i / 6) * (84 / (shelves + 1)) + 4}
					width="7"
					height={84 / (shelves + 1) - 6}
				/>
			{/each}
		{:else if kind === 'workbench'}
			<rect x="8" y="40" width="84" height="10" /><line x1="16" y1="50" x2="16" y2="90" /><line
				x1="84"
				y1="50"
				x2="84"
				y2="90"
			/>
			<path d="M30 40 L30 28 L44 28 L44 40" /><circle cx="68" cy="33" r="6" />
		{:else}
			<rect x="20" y="20" width="60" height="60" />
		{/if}
	</svg>
{/if}

<style>
	img,
	svg {
		width: 100%;
		max-width: 4rem;
		aspect-ratio: 1;
		object-fit: contain;
	}
	svg {
		fill: var(--paper);
		stroke: var(--ink);
		stroke-width: 3;
	}
	:global(.furniture) svg,
	:global(.furniture) img {
		max-width: none;
	}
	.fill {
		fill: var(--ink);
		stroke: none;
	}
</style>
