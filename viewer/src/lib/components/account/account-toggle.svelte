<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One account switch (a checkbox with its label and a hint line): saved at once through `save`; a failed save puts the
	 * box back and says so (`error`, default: the newsletter's text). `save` resolving to `false` = it didn't take (a
	 * refused browser permission): the box goes back without an error – the hint says why. The newsletter, "Show hints",
	 * "Hide sensitive content", the creator's draft preview, push (notifications-setting.svelte).
	 */
	let {
		checked,
		label,
		hint,
		save,
		testid,
		disabled = false,
		error
	}: {
		checked: boolean;
		label: string;
		hint: string;
		save: (value: boolean) => Promise<unknown> | void;
		testid?: string;
		disabled?: boolean;
		error?: string;
	} = $props();

	let saving = $state(false);
	let failed = $state(false);

	async function onchange(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const value = input.checked;
		saving = true;
		failed = false;
		try {
			if ((await save(value)) === false) input.checked = !value;
		} catch {
			input.checked = !value;
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

<label class="toggle" data-testid={testid}>
	<input type="checkbox" {checked} disabled={saving || disabled} {onchange} />
	<span>
		{label}
		<small>{hint}</small>
	</span>
</label>
{#if failed}
	<p class="error" role="alert">{error ?? m['account.marketing_opt_in_failed']()}</p>
{/if}

<style>
	.toggle {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		cursor: pointer;
	}
	.toggle input {
		margin: 0.125rem 0 0;
		width: 1.25rem;
		height: 1.25rem; /* control-ok: a checkbox */
		flex: none;
		accent-color: var(--ink);
	}
	.toggle span {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
	.error {
		border: 2px solid var(--ink);
		padding: 0.5rem;
		margin: 0;
	}
</style>
