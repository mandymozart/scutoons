<script lang="ts">
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';

	/**
	 * The catalog's top-level facts (the old files without their lists): `items` / `permissions` { version },
	 * `products` { version, currency, checked_at, source } – served in GET /api/items/ and /api/products/.
	 */
	let { data = $bindable(), id, problems }: CatalogFormProps = $props();
	const err = (...words: string[]) => problemFor(problems, ...words);
</script>

<CatalogField label={m['catalog.version']()} type="number" required value={data.version} error={err('version')} onchange={(v) => (data.version = v === null ? null : Math.floor(v))} testid="meta-version" />
{#if id === 'products'}
	<CatalogField label={m['catalog.currency']()} required value={data.currency} placeholder="EUR" error={err('currency')} onchange={(v) => (data.currency = String(v).toUpperCase())} />
	<CatalogField label={m['catalog.checked_at']()} value={data.checked_at} placeholder="2026-10-04" error={err('checked_at')} onchange={(v) => (data.checked_at = v || null)} />
	<CatalogField label={m['catalog.source']()} value={data.source} error={err('source')} onchange={(v) => (data.source = v || null)} />
{/if}
