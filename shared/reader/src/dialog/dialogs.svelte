<script lang="ts">
	import type { Dialog as DialogModel } from '@scutoons/content';
	import Dialog from './dialog.svelte';
	import type { DialogPlay } from '../types.ts';

	/**
	 * `lang` = language of the text (chapter files are per locale): drives hyphenation. `hidden` = boxes a showTextBox
	 * event reveals later (not drawn yet), `play` = a box being revealed right now (the event's effect).
	 */
	let {
		dialogs,
		lang,
		hidden,
		play
	}: {
		dialogs?: DialogModel[];
		lang?: string;
		hidden?: (dialog: DialogModel) => boolean;
		play?: (dialog: DialogModel) => DialogPlay | null | undefined;
	} = $props();
</script>

{#if dialogs?.length}
	<div class="dialogs" {lang}>
		{#each dialogs as dialog, index (dialog.id ?? index)}
			{#if !hidden?.(dialog)}
				<Dialog {dialog} play={play?.(dialog)} />
			{/if}
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
