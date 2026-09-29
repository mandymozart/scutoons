<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { SLICE_FORMAT } from '$lib/creator/defaults.ts';

	/** Image path (relative to the toon's CDN folder) with the known paths as suggestions and a live CDN preview. */
	let {
		toon,
		value,
		known = [],
		onchange,
		label = 'image',
		showFormat = false
	}: {
		toon: string;
		value: string | undefined;
		known?: string[];
		onchange: (path: string) => void;
		label?: string;
		showFormat?: boolean;
	} = $props();

	let open = $state(false);
	let filter = $state('');
	let size = $state<{ w: number; h: number } | null>(null);
	let failed = $state(false);

	const listId = $derived(`images-${label}`);
	const matches = $derived(
		known.filter((p) => !filter || p.toLowerCase().includes(filter.toLowerCase())).slice(0, 120)
	);
	const formatNote = $derived.by(() => {
		if (!size) return '';
		const { w, h } = size;
		if ((w === SLICE_FORMAT.width1x && h === SLICE_FORMAT.height1x) || (w === SLICE_FORMAT.width3x && h === SLICE_FORMAT.height3x)) return 'ok';
		if (w === SLICE_FORMAT.width1x || w === SLICE_FORMAT.width3x) return 'height';
		return 'off';
	});

	$effect(() => {
		void value;
		size = null;
		failed = false;
	});
</script>

<div class="picker">
	<label>
		<span class="lbl">{label}</span>
		<input
			type="text"
			list={listId}
			value={value ?? ''}
			placeholder="images/1x/s1c2-slice-1.jpg"
			onchange={(e) => onchange(e.currentTarget.value.trim())}
			data-testid="image-path"
		/>
	</label>
	<datalist id={listId}>
		{#each known as p}<option value={p}></option>{/each}
	</datalist>

	{#if value}
		<div class="preview">
			<img
				src={asset(`${toon}/${value}`)}
				alt=""
				onload={(e) => {
					const img = e.currentTarget as HTMLImageElement;
					size = { w: img.naturalWidth, h: img.naturalHeight };
				}}
				onerror={() => (failed = true)}
			/>
			<span class="meta">
				{#if failed}<b class="bad">not found on the CDN</b>{:else if size}{size.w} × {size.h} px{/if}
				{#if showFormat && formatNote === 'height'}<span class="warn"> · width ok, height ≠ {SLICE_FORMAT.height1x}/{SLICE_FORMAT.height3x}</span>{/if}
				{#if showFormat && formatNote === 'off'}<span class="bad"> · not {SLICE_FORMAT.width1x} / {SLICE_FORMAT.width3x} wide</span>{/if}
			</span>
		</div>
	{/if}
	{#if showFormat}<p class="hint">{m['creator.format_hint']()}</p>{/if}

	<button type="button" class="ghost browse" onclick={() => (open = !open)}>{open ? '▾' : '▸'} {known.length} known images</button>
	{#if open}
		<input type="search" placeholder="filter" bind:value={filter} />
		<ul class="grid">
			{#each matches as p (p)}
				<li>
					<button type="button" class="thumb" class:current={p === value} title={p} onclick={() => onchange(p)}>
						<img src={asset(`${toon}/${p}`)} alt="" loading="lazy" />
						<span>{p.split('/').pop()}</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.lbl {
		font-size: 10px;
		color: var(--muted);
	}
	.preview {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
	}
	.preview img {
		width: 64px;
		height: auto;
		border: 1px solid var(--border-subtle);
	}
	.meta {
		font-size: 11px;
	}
	.bad {
		color: var(--error);
	}
	.warn {
		color: var(--warn);
	}
	.browse {
		align-self: flex-start;
		font-size: 11px;
	}
	.grid {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
		gap: 0.3rem;
		max-height: 18rem;
		overflow: auto;
	}
	.thumb {
		display: flex;
		flex-direction: column;
		width: 100%;
		padding: 2px;
		background: var(--paper);
		color: var(--ink);
		border: 1px solid var(--border-subtle);
		font-size: 9px;
		text-align: left;
	}
	.thumb.current {
		border: 2px solid var(--ink);
	}
	.thumb img {
		width: 100%;
		aspect-ratio: 390 / 625;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.thumb span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
