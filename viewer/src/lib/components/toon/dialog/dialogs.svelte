<script lang="ts">
	import type { Dialog as DialogModel } from '$lib/models/content.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import Dialog from './dialog.svelte';

	let { dialogs, lang }: { dialogs?: DialogModel[]; lang?: string } = $props();

	// chapter files are loaded per locale, so the UI locale is the language of the text
	const language = $derived(lang ?? getLocale());
</script>

{#if dialogs?.length}
	<div class="dialogs" lang={language}>
		{#each dialogs as dialog, index (dialog.id ?? index)}
			<Dialog {dialog} />
		{/each}
	</div>
{/if}

<style>
	.dialogs {
		position: absolute;
		inset: 0;
		z-index: 1;
		pointer-events: none;
	}
</style>
