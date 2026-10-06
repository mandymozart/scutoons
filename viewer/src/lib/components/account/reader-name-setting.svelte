<script lang="ts">
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { READER_NAME_MAX, readerName } from '$lib/reader-name.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Account page "Your name" (agents/EXPRESSIONS.md §6): what a story's question (askText, scope "reader") stored and
	 * texts show as `{reader.name}`. Members: on the account (`PUT /me/ { display_name }`), guests: on this device.
	 * Saved when the field is left or Enter is pressed; empty clears it.
	 */
	const session = getSessionContext();
	let saving = $state(false);
	let failed = $state(false);
	let value = $state(readerName.value ?? '');
	// a name stored elsewhere (askText, another tab, the account loading) shows up here
	$effect(() => {
		const stored = readerName.value ?? '';
		if (!saving) value = stored;
	});

	async function save() {
		if (value.trim() === (readerName.value ?? '')) return;
		saving = true;
		failed = false;
		try {
			await readerName.set(value);
			value = readerName.value ?? '';
		} catch {
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

<div class="reader-name" data-testid="reader-name">
	<label for="reader-name-input">{m['account.reader_name']()}</label>
	<input
		id="reader-name-input"
		type="text"
		bind:value
		maxlength={READER_NAME_MAX}
		autocomplete="nickname"
		disabled={saving}
		onchange={save}
		onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
		data-testid="reader-name-input"
	/>
	<small>{m['account.reader_name_hint']()} {session.isLoggedIn ? m['account.reader_name_saved_account']() : m['account.reader_name_saved_device']()}</small>
	{#if failed}
		<p class="error" role="alert">{m['account.reader_name_failed']()}</p>
	{/if}
</div>

<style>
	.reader-name {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin: 0 0 1rem;
	}
	/* like the account page's section headings */
	label {
		font-size: 1rem;
		font-weight: 700;
	}
	input {
		height: 2.75rem;
		padding: 0 0.75rem;
		border: 2px solid #cccccc;
		background: white;
		color: black;
		font: inherit;
		font-size: 1rem;
		max-width: 24rem;
	}
	input:focus-visible {
		border-color: black;
		outline: none;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
	.error {
		border: 2px solid black;
		padding: 0.5rem;
		margin: 0;
	}
</style>
