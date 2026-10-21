<script lang="ts">
	import { DockHold } from '@scutoons/ui';
	import ScriptEditor from '$lib/components/script/script-editor.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A toon's big text as a page (research/KANON.md §5): a season's or a chapter's script, the toon's Kanon text – the
	 * toon's languages loaded first, then the one editor.
	 */
	let { toon, entity, title }: { toon: string; entity: string; title: string } = $props();

	const languages = $derived(loadToonLanguages(toon));
</script>

{#await languages}
	<p class="pad">{m['creator.loading']()}</p>
	<!-- the last page's Flow / Problems / Info stay docked until this page takes them over (UI-CONCEPTS E.3 b) -->
	<DockHold />
{:then langs}
	{#key `${entity}/${langs.languages.join(',')}`}
		<ScriptEditor {entity} {toon} languages={langs} {title} />
	{/key}
{:catch error}
	<p class="pad error">{error instanceof Error ? error.message : String(error)}</p>
{/await}

<style>
	.pad {
		padding: 1rem;
	}
	.error {
		color: var(--error);
	}
</style>
