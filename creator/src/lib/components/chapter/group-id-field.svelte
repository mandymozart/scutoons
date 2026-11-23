<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A group's id (its address in the chapter): typed, then committed on Enter or leaving the field. An invalid id or one
	 * another group has is refused here with the reason; a valid one goes to `onrename`, which renames it along.
	 */
	let {
		id,
		problem,
		onrename
	}: {
		id: string;
		/** why `to` can't be the id (empty, not slug-safe, taken), or null */
		problem: (to: string) => string | null;
		onrename: (to: string) => void;
	} = $props();

	const uid = $props.id();
	let text = $state('');
	let error = $state<string | null>(null);
	$effect.pre(() => {
		text = id;
		error = null;
	});

	function commit() {
		const to = text.trim();
		if (to === id) return void (error = null);
		error = problem(to);
		if (!error) onrename(to);
	}
	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') commit();
		else if (e.key === 'Escape') {
			text = id;
			error = null;
		}
	}
</script>

<div class="field" title={m['creator.group_id_hint']()} data-testid="group-id">
	<label for={uid}>{m['creator.group_id']()}</label>
	<input id={uid} type="text" bind:value={text} onchange={commit} {onkeydown} aria-invalid={error ? 'true' : undefined} spellcheck="false" autocomplete="off" />
	{#if error}<span class="error" role="alert" data-testid="group-id-error">{error}</span>{/if}
</div>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--tool-text);
	}
	input[aria-invalid='true'] {
		outline: var(--border-width-regular) solid var(--tool-stop);
	}
	.error {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--tool-stop);
	}
</style>
