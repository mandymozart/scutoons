<script lang="ts">
	import { page } from '$app/state';
	import { DockHold } from '@scutoons/ui';
	import ScriptEditor from '$lib/components/script/script-editor.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { entities } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	const toon = $derived(page.params.toonId as string);
	const languages = $derived(loadToonLanguages(toon));
</script>

{#await languages}
	<p class="pad">{m['creator.loading']()}</p>
	<!-- the last page's Flow / Problems / Info stay docked until this page takes them over (UI-CONCEPTS E.3 b) -->
	<DockHold />
{:then langs}
	{#key `${toon}/${langs.languages.join(',')}`}
		<ScriptEditor entity={entities.script(toon)} {toon} languages={langs} title={m['creator.script']()} />
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
