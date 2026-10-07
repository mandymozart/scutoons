<script lang="ts">
	// The first name a liaison / ambassador writes and signs with (owner 2026-10-07: "the first name of the liaison or
	// ambassador"): the account's `display_name` (`PUT /me/`), so the agent signs mails and DMs with it – the same name
	// readers see as `{reader.name}`. In the onboarding and on Today.
	import { m } from '$lib/paraglide/messages.js';

	let { value = $bindable(), compact = false }: { value: string; compact?: boolean } = $props();
</script>

<label class="name">
	<span class="lbl">{m['name.label']()}</span>
	<input type="text" maxlength="32" autocomplete="given-name" bind:value data-testid="writer-name" />
	{#if !compact}<span class="caption muted">{m['name.hint']()}</span>{/if}
</label>

<style>
	.name {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.lbl {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
	}
	input {
		max-width: 20rem;
	}
</style>
