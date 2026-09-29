<script lang="ts">
	import type { Dialog as DialogModel } from '@scutoons/content';
	import Dialog from './dialog.svelte';

	/** `lang` = language of the text (chapter files are per locale): drives hyphenation. */
	let { dialogs, lang }: { dialogs?: DialogModel[]; lang?: string } = $props();
</script>

{#if dialogs?.length}
	<div class="dialogs" {lang}>
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
