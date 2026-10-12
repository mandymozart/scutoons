<script lang="ts">
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import CatalogField from './catalog-field.svelte';

	/** A permission (server-only: what a code type may grant): the id and what it allows, in words */
	let { data = $bindable(), problems }: CatalogFormProps = $props();
</script>

<CatalogField label={m['catalog.description']()} required value={data.description} error={problemFor(problems, 'permission', 'description')} onchange={(v) => (data.description = v)} testid="permission-description" />
