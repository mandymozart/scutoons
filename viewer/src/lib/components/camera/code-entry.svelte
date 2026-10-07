<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';
	import { parseScutoonsCode } from './qr.ts';
	import { parseTypedCode } from './typed-code.ts';

	/**
	 * "Or type the code" (owner 2026-10-07): the code as printed for the buyer, for when the camera won't read the QR
	 * code – the same way on as a scan (`oncode`). Case is kept (codes are case-sensitive), spaces are dropped, a pasted
	 * SCUTOONS link works too. `dark` = on the camera's black picture.
	 */
	let { oncode, dark = false, id = 'code-entry' }: { oncode: (code: string) => void; dark?: boolean; id?: string } = $props();

	let value = $state('');
	let invalid = $state(false);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const code = parseTypedCode(value, parseScutoonsCode);
		invalid = !code;
		if (code) oncode(code);
	}
</script>

<form class="code-entry" class:dark onsubmit={submit} data-testid="code-entry" data-code-entry={id}>
	<label for={id}>{m['code_entry.label']()}</label>
	<div class="row">
		<input
			{id}
			type="text"
			bind:value
			oninput={() => (invalid = false)}
			placeholder={m['code_entry.placeholder']()}
			autocomplete="off"
			autocapitalize="off"
			autocorrect="off"
			spellcheck="false"
			maxlength="200"
			aria-invalid={invalid}
			aria-describedby={invalid ? `${id}-invalid` : undefined}
		/>
		<button type="submit" disabled={!value.trim()}>{m['code_entry.submit']()}</button>
	</div>
	{#if invalid}<p class="invalid" id="{id}-invalid" role="alert">{m['code_entry.invalid']()}</p>{/if}
</form>

<style>
	.code-entry {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
	}
	label {
		font-size: 0.875rem;
	}
	/* one line: the field and its button the same height, edges aligned */
	.row {
		display: flex;
		align-items: stretch;
		gap: 0.5rem;
		min-width: 0;
	}
	input,
	button {
		margin: 0;
		box-sizing: border-box;
		height: var(--control-height);
		min-height: 0;
		font-family: var(--font-mono);
		font-size: 1rem;
		line-height: 1;
		vertical-align: top;
	}
	input {
		flex: 1 1 auto;
		min-width: 0;
		padding: 0 0.75rem;
		border: 2px solid #000000;
		border-radius: 0;
		background: #ffffff;
		color: #000000;
		/* codes are case-sensitive: never shown in other letters than typed */
		text-transform: none;
		letter-spacing: 0.05em;
	}
	button {
		flex: none;
		padding: 0 1rem;
		border: 2px solid #000000;
	}
	.invalid {
		margin: 0;
		font-size: 0.75rem;
		line-height: 1.3;
		text-transform: none;
	}
	.dark label,
	.dark .invalid {
		color: inherit;
	}
	.dark input {
		border-color: currentColor;
		background: rgb(0 0 0 / 0.55);
		color: #ffffff;
	}
	.dark button {
		border: 2px solid currentColor;
		background: #ffffff;
		color: #000000;
	}
</style>
