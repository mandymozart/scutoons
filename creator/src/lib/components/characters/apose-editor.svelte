<script lang="ts">
	import { aposeConventionPath, conventionName, type CharacterLook } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { MediaSlot } from '@scutoons/media';

	/**
	 * The A-pose of one look (owner 2026-10-12: "a poses come per look") – the reader's avatar body in this look – as an
	 * image slot (owner 2026-10-03: "the standard A-pose is a case for the asset picker"): empty until a file is picked or
	 * uploaded (the picker suggests the persona's folder; the convention name is the tooltip), × clears it. The look's
	 * wear slots (`apose.slots`: id, type, label – a duo has a type twice) are data only: worn items are not drawn on
	 * the body, they sit in the reader's avatar page slots (owner 2026-10-13: no image px any more).
	 */
	let {
		character,
		persona,
		look,
		bust = 0,
		onchange
	}: {
		character: string;
		persona: string;
		look: CharacterLook;
		/** set after an import (a timestamp): previews load again */
		bust?: number;
		/** a file picked (a CDN-root path) or cleared (undefined) */
		onchange: (path: string | undefined) => void;
	} = $props();
</script>

<section class="apose" data-testid="apose-editor" title={look.apose?.path || aposeConventionPath(character, persona, look.id)}>
	<MediaSlot
		toon=""
		value={look.apose?.path?.trim() || undefined}
		label={m['creator.characters_apose']()}
		place="characters/{conventionName(character, persona)}/"
		{bust}
		{onchange}
		testid="apose-{persona}-{look.id}-slot"
	/>
</section>

<style>
	.apose {
		width: 14rem;
	}
</style>
