<script lang="ts">
	// Account page "Saved chapters" (PWA phase 3, agents/research/PWA.md §2 "Phase 3"): what is saved on this device,
	// the space it takes (and what the browser allows), one "Remove" per chapter and "Remove all". Nothing when nothing
	// is saved (saving happens on the chapter's row).
	import { confirmDialog, Icon } from '@scutoons/ui';
	import { onMount } from 'svelte';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { savedChapters } from '$lib/offline/saved-chapters.svelte.ts';
	import { AUTO_SAVE_SETTINGS, roundMb } from '$lib/sw/saved-rules.ts';
	import { autoSave } from '$lib/offline/auto-save.svelte.ts';

	// the default: save a chapter by itself when it is opened (owner 2026-10-16) – Only by hand / On Wi-Fi / Always
	let canSave = $state(false);
	let knowsWifi = $state(true);

	let estimate = $state<{ usage: number; quota: number } | null>(null);
	const count = $derived(savedChapters.list.length);

	async function measure() {
		const e = await navigator.storage?.estimate?.().catch(() => undefined);
		estimate = e?.usage !== undefined && e.quota ? { usage: e.usage, quota: e.quota } : null;
	}
	onMount(() => {
		canSave = 'serviceWorker' in navigator && 'caches' in window;
		knowsWifi = !!(navigator as Navigator & { connection?: { type?: string } }).connection?.type;
		void savedChapters.refresh().then(measure);
	});

	async function remove(toon: string, chapter: number, title: string) {
		if (!(await confirmDialog({ title: m['saved.remove_title']({ chapter: title }), confirmLabel: m['saved.remove'](), cancelLabel: m['common.cancel']() }))) return;
		await savedChapters.remove(toon, chapter);
		await measure();
	}

	async function removeAll() {
		if (!(await confirmDialog({ title: m['saved.remove_all_title'](), confirmLabel: m['saved.remove_all'](), cancelLabel: m['common.cancel']() }))) return;
		await savedChapters.removeAll();
		await measure();
	}
</script>

{#if canSave}
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
{#if count}
	<fieldset class="saved-chapters" data-testid="saved-chapters-setting">
		<legend>{m['saved.title']()}</legend>
		<ul>
			{#each savedChapters.list as saved (saved.toon + '/' + saved.chapter)}
				{@const name = `${saved.toonTitle ?? saved.toon} · ${saved.label ?? m['common.chapter']({ number: saved.chapter })}`}
				<li>
					<a href={localizeHref(`/${saved.toon}/chapter/${saved.chapter}`)}>{name}{saved.title ? ` · ${saved.title}` : ''}</a>
					<small>{roundMb(saved.bytes)} MB</small>
					<button type="button" class="round" title={m['saved.remove']()} aria-label={m['saved.remove']()} onclick={() => remove(saved.toon, saved.chapter, name)}>
						<Icon name="trash-2" size="1rem" />
					</button>
				</li>
			{/each}
		</ul>
		<p>
			<button type="button" class="btn-secondary" onclick={removeAll} data-testid="saved-remove-all">{m['saved.remove_all']()}</button>
			<small>
				{m['saved.summary']({ count, size: roundMb(savedChapters.bytes) })}
				{#if estimate}· {m['saved.storage']({ used: roundMb(estimate.usage), quota: roundMb(estimate.quota) })}{/if}
			</small>
		</p>
	</fieldset>
{/if}

<style>
	/* the options like the media quality's (media-quality-setting.svelte): one row of square radio buttons */
	.auto-save {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.options {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.options label {
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
	.options label:first-child {
		margin-left: 0;
	}
	.options label:hover,
	.options label:has(input:focus-visible) {
		border-color: black;
		z-index: 1;
	}
	.options label.active {
		background: black;
		color: white;
		border-color: black;
		z-index: 1;
	}
	.options input {
		position: absolute;
		opacity: 0;
		inset: 0;
		margin: 0;
		cursor: pointer;
	}
	.saved-chapters {
		border: 0;
		margin: 0 0 1rem;
		padding: 0;
		min-width: 0;
	}
	/* like the account page's section headings (fx-setting.svelte) */
	legend {
		padding: 0;
		margin-bottom: 0.75rem;
		font-size: 1rem;
		font-weight: 700;
	}
	ul {
		list-style: none;
		margin: 0 0 0.75rem;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.25rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	li a {
		flex: 1 1 auto;
		min-width: 0;
		color: inherit;
	}
	p {
		margin: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 0.75rem;
	}
	.btn-secondary {
		height: var(--control-height);
		padding: 0 1rem;
	}
	/* a standalone icon-only button: round */
	.round {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
	}
	small {
		font-size: 0.75rem;
		line-height: 1.3;
		color: #6b6b6b;
	}
</style>
