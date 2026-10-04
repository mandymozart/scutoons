<script lang="ts">
	/**
	 * The "Advanced" part of a catalog form: the entry's other fields (skills, effects, wear, grantable …) as JSON in a
	 * textarea. Every valid edit goes to `onchange` at once; an invalid one shows the parse error and changes nothing.
	 */
	let {
		label,
		hint,
		value,
		onchange,
		testid
	}: {
		label: string;
		hint?: string;
		value: Record<string, unknown>;
		onchange: (next: Record<string, unknown>) => void;
		testid?: string;
	} = $props();

	const format = (v: unknown) => JSON.stringify(v, null, 2);
	let text = $state('');
	let error = $state<string | null>(null);
	/** what this field last handed out / showed – a new `value` from outside (another entry, undo) replaces the text */
	let shown = '';

	$effect(() => {
		const next = format(value);
		if (next !== shown) {
			shown = next;
			text = next;
			error = null;
		}
	});

	function input(next: string) {
		text = next;
		try {
			const parsed = JSON.parse(next.trim() === '' ? '{}' : next);
			if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('{ … }');
			error = null;
			shown = format(parsed);
			onchange(parsed);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	const rows = $derived(Math.min(24, Math.max(4, text.split('\n').length + 1)));
</script>

<label class="json">
	<span class="sc-label">{label}</span>
	<textarea {rows} spellcheck="false" value={text} oninput={(e) => input(e.currentTarget.value)} aria-invalid={!!error} data-testid={testid}></textarea>
	{#if error}<span class="error" role="alert">{error}</span>{:else if hint}<span class="sc-caption muted">{hint}</span>{/if}
</label>

<style>
	.json {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	textarea {
		font-family: var(--font-mono, monospace);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: pre;
		overflow: auto;
	}
	.error {
		color: var(--state-error);
		font-size: var(--type-caption-size);
	}
</style>
