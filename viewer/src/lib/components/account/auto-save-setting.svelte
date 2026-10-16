<script lang="ts">
	// Account page → Offline-Inhalte → "Save chapters by themselves" (PWA phase 3, lib/offline/auto-save.svelte.ts;
	// ACCOUNT-PAGE.md §0): Only by hand / On Wi-Fi / Always as the square radio row (media-quality-setting.svelte's).
	// Nothing where this browser can't save chapters (no service worker controlling the page).
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { autoSave } from '$lib/offline/auto-save.svelte.ts';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { AUTO_SAVE_SETTINGS } from '$lib/sw/saved-rules.ts';

	/** Android reports the connection type; elsewhere "On Wi-Fi" can't know – the hint says so */
	let knowsWifi = $state(true);
	onMount(() => {
		knowsWifi = !!(navigator as Navigator & { connection?: { type?: string } }).connection?.type;
	});
</script>

{#if savedChapters.supported}
	<fieldset class="auto-save" data-testid="auto-save-setting">
		<legend>{m['saved.auto_title']()}</legend>
		<div class="options">
			{#each AUTO_SAVE_SETTINGS as value (value)}
				<label class:active={autoSave.value === value}>
					<input type="radio" name="auto-save" {value} checked={autoSave.value === value} onchange={() => autoSave.set(value)} data-testid={`auto-save-${value}`} />
					{m[`saved.auto_${value}`]()}
				</label>
			{/each}
		</div>
		<small>{m['saved.auto_hint']()}{#if autoSave.value === 'wifi' && !knowsWifi} {m['saved.auto_wifi_unknown']()}{/if}</small>
	</fieldset>
{/if}

<style>
	.auto-save {
		border: 0;
		margin: 0;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	/* the square radio row (media-quality-setting.svelte): the chosen one inverted */
	.options {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	label {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: var(--control-height);
		box-sizing: border-box;
		padding: 0.25rem;
		border: var(--border-width-regular) solid var(--border-muted);
		margin-left: calc(-1 * var(--border-width-regular));
		text-align: center;
		font-size: 0.75rem;
		line-height: 1.2;
		cursor: pointer;
		overflow-wrap: anywhere;
	}
	label:first-child {
		margin-left: 0;
	}
	label:hover,
	label:has(input:focus-visible) {
		border-color: var(--ink);
		z-index: 1;
	}
	label.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
		z-index: 1;
	}
	input {
		position: absolute;
		opacity: 0;
		inset: 0;
		margin: 0;
		cursor: pointer;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: var(--text-muted);
	}
</style>
