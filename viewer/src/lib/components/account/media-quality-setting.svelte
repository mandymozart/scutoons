<script lang="ts">
	import { MEDIA_QUALITY_SETTINGS, type MediaQualitySetting } from '@scutoons/content';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { mediaQuality } from '$lib/media-quality.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Account page "Media quality" (agents/DESIGN.md "Media quality"): Automatic / Low / Medium / High as one row of
	 * square radio buttons. Members: stored in the account (`PUT /me/`), guests: on this device (localStorage).
	 */
	const session = getSessionContext();
	let saving = $state(false);
	let failed = $state(false);
	const current = $derived(mediaQuality.setting ?? 'auto');
	const label = (value: MediaQualitySetting) => m[`media_quality.${value}`]();

	async function choose(value: MediaQualitySetting) {
		if (value === current) return;
		saving = true;
		failed = false;
		try {
			await mediaQuality.set(value);
		} catch {
			failed = true;
		} finally {
			saving = false;
		}
	}
</script>

<fieldset class="media-quality" data-testid="media-quality" disabled={saving}>
	<legend>{m['media_quality.title']()}</legend>
	<div class="options">
		{#each MEDIA_QUALITY_SETTINGS as value (value)}
			<label class:active={current === value}>
				<input type="radio" name="media-quality" {value} checked={current === value} onchange={() => choose(value)} data-testid={`media-quality-${value}`} />
				{label(value)}
			</label>
		{/each}
	</div>
	<small>
		{m['media_quality.hint']()}
		{#if current === 'auto' && mediaQuality.auto}{m['media_quality.auto_now']({ level: label(mediaQuality.auto) })}{/if}
		{session.isLoggedIn ? m['media_quality.saved_account']() : m['media_quality.saved_device']()}
	</small>
	{#if failed}
		<p class="error" role="alert">{m['media_quality.failed']()}</p>
	{/if}
</fieldset>

<style>
	.media-quality {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	/* like the account page's section headings */
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	.options {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}
	label {
		position: relative;
		display: block;
		padding: 0.5rem 0.25rem;
		border: 2px solid #cccccc;
		margin-left: -2px;
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
		border-color: black;
		z-index: 1;
	}
	label.active {
		background: black;
		color: white;
		border-color: black;
		z-index: 1;
	}
	input {
		position: absolute;
		opacity: 0;
		inset: 0;
		margin: 0;
		cursor: pointer;
	}
	fieldset:disabled label {
		opacity: 0.5;
		cursor: not-allowed;
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
