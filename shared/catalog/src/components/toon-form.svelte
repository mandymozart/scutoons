<script lang="ts">
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';

	/**
	 * A toon (owner: main toons are created by admins only, here; the creator then fills content and meta): its name and
	 * where its QR codes lead (redirectUrl). A new toon also gets its draft content entry, so the creator lists it.
	 */
	let { data = $bindable(), creating, problems }: CatalogFormProps = $props();
	const err = (...words: string[]) => problemFor(problems, ...words);
</script>

<CatalogField label={m['catalog.name']()} required value={data.name} error={err('name')} onchange={(v) => (data.name = v)} testid="toon-name" />
<CatalogField label={m['catalog.redirect_url']()} type="url" required value={data.redirectUrl} placeholder="https://scutoons.com/…" hint={m['catalog.redirect_hint']()} error={err('redirectUrl')} onchange={(v) => (data.redirectUrl = v)} testid="toon-redirect" />
{#if creating}<p class="sc-caption muted">{m['catalog.toon_new_hint']()}</p>{/if}

<style>
	p {
		margin: 0;
	}
</style>
