<script lang="ts">
	import { Select } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { SOURCE_MARK, textLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * The text language (the edit language): which ONE content language the translatable fields show – the source by
	 * default, marked with the star; a translation shows its original under each field. Not the interface language. The
	 * kit's select: boxed in the ☰ menu, `quiet` in the translator's bar.
	 */
	let {
		languages,
		source,
		quiet = false
	}: {
		/** the content languages to offer (the toon's, source first) */
		languages: string[];
		/** the source language (the default, marked): the toon's, or the platform's on pages without a toon */
		source: string;
		quiet?: boolean;
	} = $props();

	const current = $derived(textLanguage.current(languages, source));
	const options = $derived(languages.map((l) => ({ value: l, label: `${l.toUpperCase()}${l === source ? ` ${SOURCE_MARK}` : ''}` })));
	const title = $derived(`${m['shell.text_language']()}: ${m['creator.text_language_hint']()}`);
</script>

<Select value={current} {options} {quiet} label={m['shell.text_language']()} {title} onchange={(v) => textLanguage.set(v)} data-testid="text-language-select" />
