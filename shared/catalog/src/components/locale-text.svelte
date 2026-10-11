<script lang="ts">
	import { catalogLocales } from '../setup.ts';
	const locales = catalogLocales();

	/**
	 * A catalog text in every language ({ de, en }): one field group (legend = the label, no frame – tools.css), one
	 * line per language. A plain string (older entries) is shown in every language and saved as { de, en }.
	 */
	let {
		label,
		value,
		multiline = false,
		required = false,
		error = null,
		onchange,
		testid
	}: {
		label: string;
		value: unknown;
		multiline?: boolean;
		required?: boolean;
		error?: string | null;
		onchange: (next: Record<string, string>) => void;
		testid?: string;
	} = $props();

	const current = $derived<Record<string, string>>(
		typeof value === 'string' ? Object.fromEntries(locales.map((l) => [l, value])) : value && typeof value === 'object' ? (value as Record<string, string>) : {}
	);

	function set(locale: string, text: string) {
		onchange({ ...Object.fromEntries(locales.map((l) => [l, current[l] ?? ''])), ...current, [locale]: text });
	}
</script>

<fieldset class="locale-text" data-testid={testid}>
	<legend>{label}{#if required}<span aria-hidden="true"> *</span>{/if}</legend>
	{#each locales as locale (locale)}
		<label class="lang">
			<span class="sc-caption">{locale.toUpperCase()}</span>
			{#if multiline}
				<textarea rows="3" value={current[locale] ?? ''} oninput={(e) => set(locale, e.currentTarget.value)} aria-invalid={!!error} data-testid={testid ? `${testid}-${locale}` : undefined}></textarea>
			{:else}
				<input type="text" value={current[locale] ?? ''} oninput={(e) => set(locale, e.currentTarget.value)} aria-invalid={!!error} data-testid={testid ? `${testid}-${locale}` : undefined} />
			{/if}
		</label>
	{/each}
	{#if error}<p class="sc-error" role="alert">{error}</p>{/if}
</fieldset>

<style>
	.locale-text {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-15);
	}
	.lang {
		display: grid;
		grid-template-columns: 2rem minmax(0, 1fr);
		align-items: center;
		gap: var(--space-05);
	}
	.lang textarea {
		align-self: stretch;
	}
</style>
