<script lang="ts">
	import type { Dialog as DialogModel } from '@scutoons/content';
	import Dialog from './dialog.svelte';
	import type { DialogPlay, TextExpressions } from '../types.ts';

	/**
	 * `lang` = language of the text (chapter files are per locale): drives hyphenation. `hidden` = boxes a showTextBox
	 * event reveals later (not drawn yet), `play` = a box being revealed right now (the event's effect). `expressions` =
	 * the reader's answers for `{…}` in the texts (none: the texts as written).
	 */
	let {
		dialogs,
		lang,
		hidden,
		play,
		expressions
	}: {
		dialogs?: DialogModel[];
		lang?: string;
		hidden?: (dialog: DialogModel) => boolean;
		play?: (dialog: DialogModel) => DialogPlay | null | undefined;
		expressions?: TextExpressions;
	} = $props();
</script>

{#if dialogs?.length}
	<div class="dialogs" {lang}>
		{#each dialogs as dialog, index (dialog.id ?? index)}
			{#if !hidden?.(dialog)}
				<Dialog {dialog} play={play?.(dialog)} {expressions} {lang} />
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
