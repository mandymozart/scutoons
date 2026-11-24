<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** The Library's Find field (the panel's and the picker's): its icon, the words typed, Ctrl L to come here (the owner's key handler focuses `input`). */
	let { value = $bindable(''), input = $bindable(null), autofocus = false, testid }: { value?: string; input?: HTMLInputElement | null; autofocus?: boolean; testid: string } = $props();
</script>

<span class="find">
	<Icon name="search" size="0.9rem" />
	<!-- svelte-ignore a11y_autofocus -->
	<input bind:this={input} type="search" bind:value placeholder={m['library.find']()} aria-label={m['library.find']()} title={m['library.find_title']()} {autofocus} data-testid={testid} />
</span>

<style>
	.find {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 0.35rem;
		min-width: 0;
		height: var(--control-height);
		padding: 0 var(--editor-pad);
		border: var(--editor-edge) solid var(--kit-field-rest-border);
		border-radius: var(--editor-radius);
		background: var(--kit-field-rest-ground);
		color: var(--tool-muted);
	}
	.find:focus-within {
		border-color: var(--kit-field-on-border);
	}
	.find input {
		flex: 1 1 auto;
		min-width: 0;
		height: 100%; /* control-ok: fills the Find field, which is a control high */
		padding: 0;
		border: 0;
		background: none;
		color: var(--tool-text);
		font: inherit;
		outline: none;
	}
</style>
