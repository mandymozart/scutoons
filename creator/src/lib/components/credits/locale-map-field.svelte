<script lang="ts">
	import type { LocaleText } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/** A `{ <locale>: text }` value (role label, song title, bio …): one input per toon language, missing ones flagged. */
	let {
		value,
		languages,
		label,
		required = false,
		multiline = false,
		testid,
		onchange
	}: {
		value: LocaleText | undefined;
		languages: string[];
		label: string;
		required?: boolean;
		multiline?: boolean;
		testid?: string;
		onchange: (value: LocaleText) => void;
	} = $props();

	const anyText = $derived(Object.values(value ?? {}).some((t) => t?.trim()));

	function set(locale: string, text: string) {
		onchange({ ...(value ?? {}), [locale]: text });
	}
</script>

<fieldset class="locale-map" data-testid={testid}>
	<legend>{label}</legend>
	{#each languages as l (l)}
		{@const missing = !value?.[l]?.trim() && (required || anyText)}
		<label class:missing>
			<span class="lang">{l}{#if missing}<span class="pill">{m['creator.missing']()}</span>{/if}</span>
			{#if multiline}
				<textarea rows="3" value={value?.[l] ?? ''} oninput={(e) => set(l, e.currentTarget.value)}></textarea>
			{:else}
				<input type="text" value={value?.[l] ?? ''} oninput={(e) => set(l, e.currentTarget.value)} data-testid={testid ? `${testid}-${l}` : undefined} />
			{/if}
		</label>
	{/each}
</fieldset>

<style>
	.locale-map {
		border: 0;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: 0.25rem 0.5rem;
		min-width: 0;
	}
	legend {
		font-size: 11px;
		font-weight: 700;
		padding: 0;
		margin-bottom: 0.15rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}
	.lang {
		font-size: 11px;
		color: var(--text-muted);
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.missing input,
	.missing textarea {
		border-color: var(--warn);
	}
	.pill {
		font-size: 10px;
		padding: 0 0.3em;
		background: var(--warn);
		color: var(--ink);
	}
	input,
	textarea {
		width: 100%;
		box-sizing: border-box;
	}
</style>
