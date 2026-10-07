<script lang="ts">
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { ActionBar, Badge, Button, CloseIcon, Section, Skeleton, SortGrip, sortable } from '@scutoons/ui';
	import type { AdminFeatured } from '@scutoons/api';
	import { localized, pickVariant, type FeaturedProduct, type Product, type ProductCatalog } from '@scutoons/content';
	import { admin, fetchProducts } from '$lib/api.ts';
	import { CDN_SERVER, VIEWER_URL } from '$lib/config.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { getToaster } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/**
	 * The landing page's limited editions (GET/PUT /api/admin/featured, owner 2026-10-01): which products of the shop
	 * catalog, in which colour (= which images), in which order. Stored server-side; the public /products/featured/ reads it.
	 */
	const toaster = getToaster();
	const locale = getLocale();

	let current = $state<AdminFeatured | null>(null);
	let catalog = $state<ProductCatalog | null>(null);
	/** the list being edited */
	let draft = $state<FeaturedProduct[]>([]);
	let error = $state<string | null>(null);
	let saving = $state(false);
	let adding = $state('');

	function load() {
		error = null;
		Promise.all([admin.featured(), fetchProducts()])
			.then(([featured, products]) => {
				catalog = products;
				apply(featured);
			})
			.catch((e) => (error = errorText(e)));
	}

	function apply(next: AdminFeatured) {
		current = next;
		draft = next.items.map((item) => ({ ...item }));
	}

	load();

	const productOf = (id: string): Product | undefined => catalog?.products.find((p) => p.id === id);
	const same = (a: FeaturedProduct[], b: FeaturedProduct[]) =>
		a.length === b.length && a.every((x, i) => x.product === b[i].product && (x.variant ?? '') === (b[i].variant ?? ''));
	const changed = $derived(current !== null && !same(draft, current.items));
	const max = $derived(current?.max ?? 12);

	function move(from: number, to: number) {
		const next = [...draft];
		const [item] = next.splice(from, 1);
		next.splice(to, 0, item);
		draft = next;
	}

	function setVariant(index: number, variant: string) {
		draft[index] = variant ? { product: draft[index].product, variant } : { product: draft[index].product };
	}

	function add() {
		const product = productOf(adding);
		if (!product || draft.length >= max) return;
		// the first colour that is not in the list yet (and not sold out), else the product's default
		const taken = new Set(draft.filter((d) => d.product === product.id).map((d) => d.variant ?? ''));
		const variant = product.variants.find((v) => !v.soldOut && !taken.has(v.id)) ?? product.variants.find((v) => !taken.has(v.id));
		draft = [...draft, variant ? { product: product.id, variant: variant.id } : { product: product.id }];
		adding = '';
	}

	async function save(items: FeaturedProduct[]) {
		if (!changed) {
			toaster.addToast({ message: m['homepage.nothing_changed'](), type: 'info' });
			return;
		}
		saving = true;
		try {
			apply(await admin.saveFeatured(items));
			toaster.addToast({ message: m['homepage.saved'](), type: 'success' });
		} catch (e) {
			toaster.addToast({ message: errorText(e), type: 'error' });
		} finally {
			saving = false;
		}
	}

	const icon = (path: string | null | undefined) => (path ? `${CDN_SERVER}/${path.replace(/^\/+/, '')}` : '');
	const homepage = `${VIEWER_URL}/${locale === 'de' ? '' : `${locale}/`}`;
</script>

<PageHeader type={m['nav.group_data']()} title={m['homepage.title']()} meta={m['homepage.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['homepage.title']()}>
	{#snippet info()}<PageHelp page="homepage" />{/snippet}
	{#snippet start()}
		{#if changed}<span class="unsaved">{m['homepage.unsaved']()}</span>
		{:else if current?.updated_at}
			<span class="muted">{m['homepage.last_change']({ at: minute(current.updated_at), by: (current.updated_by ?? '—').replace(/^admin:/, '') })}</span>
		{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" variant="secondary" href={homepage} target="_blank" rel="noopener">{m['homepage.preview']()}</Button>
		<Button size="sm" onclick={() => save($state.snapshot(draft))} disabled={!current || saving || !changed} data-testid="featured-save">
			{saving ? m['homepage.saving']() : m['homepage.save']()}
		</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !current || !catalog}
	<Skeleton lines={6} label={m['shell.loading']()} />
{:else}
	<Section title={m['homepage.section_featured']()} hint={m['homepage.section_featured_hint']({ max })}>
		{#if draft.length}
			<ol class="featured" use:sortable={{ onmove: move }} data-testid="featured-list">
				{#each draft as item, i (`${item.product}/${item.variant ?? ''}/${i}`)}
					{@const product = productOf(item.product)}
					{@const shown = product ? pickVariant(product, item.variant) : null}
					<li class="row" data-testid="featured-row" data-product={item.product} data-variant={item.variant ?? ''} data-sort-row>
						<SortGrip label={m['common.move']({ name: product?.name ?? item.product })} title={m['common.move_hint']()} />
						<span class="pos">{i + 1}</span>
						{#if product}
							<img class="icon" src={icon(shown?.icon ?? product.icon)} alt="" loading="lazy" />
							<span class="name">
								<span>{product.name}</span>
								<span class="muted">{product.id}</span>
							</span>
							<!-- only when there is a choice (owner 2026-10-02: a one-variant product showed a pointless "Colour" select) -->
							{#if product.variants.length > 1}
							<label class="variant">
								<span class="muted">{m['homepage.variant']()}</span>
								<select value={item.variant ?? ''} onchange={(e) => setVariant(i, e.currentTarget.value)} data-testid="featured-variant">
									<option value="">{m['homepage.variant_default']({ name: localized(pickVariant(product).name, locale) })}</option>
									{#each product.variants as v (v.id)}
										<option value={v.id}>{localized(v.name, locale)}{v.soldOut ? ` (${m['homepage.sold_out']()})` : ''}</option>
									{/each}
								</select>
							</label>
							{/if}
							{#if shown?.soldOut}<Badge>{m['homepage.sold_out']()}</Badge>{/if}
						{:else}
							<span class="name error">{m['homepage.unknown']({ product: item.product })}</span>
						{/if}
						<span class="tools">
							<Button size="sm" variant="secondary" onclick={() => (draft = draft.filter((_, j) => j !== i))} aria-label={m['homepage.remove']()} title={m['homepage.remove']()} data-testid="featured-remove"><CloseIcon /></Button>
						</span>
					</li>
				{/each}
			</ol>
		{/if}
		<form
			class="add"
			onsubmit={(event) => {
				event.preventDefault();
				add();
			}}
		>
			<label>
				<span class="muted">{m['homepage.add_product']()}</span>
				<select bind:value={adding} data-testid="featured-add-product">
					<option value="">{m['homepage.choose']()}</option>
					{#each catalog.products as p (p.id)}
						<option value={p.id}>{p.name}</option>
					{/each}
				</select>
			</label>
			<Button size="sm" type="submit" variant="secondary" disabled={!adding || draft.length >= max} data-testid="featured-add">{m['homepage.add']()}</Button>
		</form>
	</Section>
{/if}

<style>
	.featured {
		list-style: none;
		margin: 0 0 var(--space-15);
		padding: 0;
		display: flex;
		flex-direction: column;
		max-width: 52rem;
	}
	.row {
		display: grid;
		grid-template-columns: auto 1.5rem 3rem minmax(0, 1fr) auto auto auto;
		align-items: center;
		gap: var(--space-075, 0.75rem);
		padding: var(--space-05, 0.5rem) 0;
		border-bottom: 1px solid var(--border-subtle, #ddd);
	}
	.pos {
		color: var(--text-muted);
		text-align: right;
	}
	.icon {
		width: 3rem;
		height: 3rem;
		object-fit: contain;
	}
	.name {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.variant,
	.add label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.tools {
		display: flex;
		gap: 0.25rem;
	}
	.add {
		display: flex;
		align-items: flex-end;
		gap: var(--space-05, 0.5rem);
	}
	.muted {
		color: var(--text-muted);
	}
	.unsaved {
		font-weight: var(--font-weight-regular);
	}
	.error {
		color: var(--error, #c00);
	}
	@media (max-width: 640px) {
		.row {
			grid-template-columns: auto 1.5rem 3rem minmax(0, 1fr);
		}
		.variant,
		.tools {
			grid-column: 4;
		}
	}
</style>
