<script lang="ts">
	/**
	 * URL-driven pager for server-side lists (`{ total, page, pageSize }` of the admin API). Renders links built by
	 * `href(page)` / `sizeHref(pageSize)`, so the URL stays the single source of the list state.
	 */
	let {
		page,
		pageSize,
		total,
		href,
		sizeHref,
		pageSizes = [25, 50, 100, 250],
		compact = false,
		labels = {
			label: 'Pages',
			previous: 'Previous',
			next: 'Next',
			range: (from: number, to: number, count: number) => `${from}–${to} of ${count}`,
			pageSize: 'Per page'
		}
	}: {
		page: number;
		pageSize: number;
		total: number;
		href: (page: number) => string;
		/** link for another page size (shown as a small choice when given) */
		sizeHref?: (pageSize: number) => string;
		pageSizes?: number[];
		/** one line without wrapping (inside the ActionBar) */
		compact?: boolean;
		labels?: {
			label: string;
			previous: string;
			next: string;
			range: (from: number, to: number, total: number) => string;
			pageSize: string;
		};
	} = $props();

	const pages = $derived(Math.max(1, Math.ceil(total / Math.max(1, pageSize))));
	const from = $derived(total === 0 ? 0 : (page - 1) * pageSize + 1);
	const to = $derived(Math.min(total, page * pageSize));

	/** 1 … 4 5 [6] 7 8 … 20 */
	const numbers = $derived.by(() => {
		const list: (number | null)[] = [];
		for (let p = 1; p <= pages; p++) {
			if (p === 1 || p === pages || Math.abs(p - page) <= 2) list.push(p);
			else if (list[list.length - 1] !== null) list.push(null);
		}
		return list;
	});
</script>

<nav class="sc-pager" class:compact aria-label={labels.label}>
	<span class="range">{labels.range(from, to, total)}</span>
	{#if pages > 1}
		<span class="pages">
			{#if page > 1}
				<a href={href(page - 1)} rel="prev">{labels.previous}</a>
			{/if}
			{#each numbers as number, i (i)}
				{#if number === null}
					<span class="gap" aria-hidden="true">…</span>
				{:else if number === page}
					<span class="current" aria-current="page">{number}</span>
				{:else}
					<a href={href(number)}>{number}</a>
				{/if}
			{/each}
			{#if page < pages}
				<a href={href(page + 1)} rel="next">{labels.next}</a>
			{/if}
		</span>
	{/if}
	{#if sizeHref}
		<span class="sizes">
			{labels.pageSize}
			{#each pageSizes as size (size)}
				{#if size === pageSize}
					<span class="current" aria-current="true">{size}</span>
				{:else}
					<a href={sizeHref(size)}>{size}</a>
				{/if}
			{/each}
		</span>
	{/if}
</nav>

<style>
	.sc-pager {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-05) var(--space-1);
		padding: var(--space-05) 0;
		font-variant-numeric: tabular-nums;
	}
	.compact {
		flex-wrap: nowrap;
		padding: 0;
		white-space: nowrap;
	}
	.compact .pages,
	.compact .sizes {
		flex-wrap: nowrap;
	}
	.range {
		color: var(--text-muted);
	}
	.pages,
	.sizes {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-025);
	}
	.sizes {
		color: var(--text-muted);
	}
	a,
	.current,
	.gap {
		display: inline-block;
		min-width: 1.8em;
		padding: 0.2em 0.45em;
		text-align: center;
		box-sizing: border-box;
	}
	a {
		color: var(--ink);
		border: var(--border-width-hairline) solid var(--border-muted);
		text-decoration: none;
	}
	a:hover,
	a:focus-visible {
		border-color: var(--ink);
	}
	.current {
		background: var(--ink);
		color: var(--paper);
		border: var(--border-width-hairline) solid var(--ink);
	}
</style>
