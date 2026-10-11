<script lang="ts">
	import { page } from '$app/state';
	import CatalogEditor from '$lib/components/catalog/catalog-editor.svelte';
	import { isCatalogPageKind } from '$lib/editor/catalog-docs.ts';

	const kind = $derived(isCatalogPageKind(page.params.kind) ? page.params.kind : 'items');
</script>

{#key kind}
	<CatalogEditor {kind} />
{/key}
