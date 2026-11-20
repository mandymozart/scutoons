<script lang="ts">
	import { onMount } from 'svelte';
	import { Section, Toggle } from '@scutoons/ui/editor';
	import { PANELS_KEY } from '@scutoons/ui/editor';
	import { browserWorkspace } from '$lib/editor/storyboard-workspace.ts';
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { SCALE_MAX, SCALE_MIN, SCALE_STEP } from '$lib/shell/settings.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * Settings as a floating panel (research/redesign/PLAN.md §2.5, the deck's settings.rs): this machine's look, motion
	 * and layout and the local folder are kept in this browser; what belongs to the member (AI, notifications) shows
	 * where an API is there and says "later" where not. Nothing to save: every change holds at once.
	 */
	let { onreset }: { onreset: () => void } = $props();

	const s = $derived(shell.settings);
	const percent = $derived(`${Math.round(s.scale * 100)} %`);

	const folder = browserWorkspace();
	let folderName = $state<string | null>(null);
	onMount(() => {
		void folder.restore().then(() => (folderName = folder.name));
	});
	async function chooseFolder() {
		try {
			if (await folder.choose()) folderName = folder.name;
		} catch {
			// the browser refused the folder: the old one stays
		}
	}
	function resetAll() {
		try {
			localStorage.removeItem(PANELS_KEY);
		} catch {
			// storage blocked: only this workspace resets
		}
		onreset();
	}
</script>

<div class="settings" data-testid="settings">
	<Section id="shell-look" label={m['shell.look']()} summary={m['shell.this_machine']()}>
		<div class="row">
			<span>{m['shell.theme']()}</span>
			<span class="pair" role="group" aria-label={m['shell.theme']()}>
				<button type="button" class="pill" aria-pressed={s.theme === 'day'} onclick={() => shell.change({ theme: 'day' })} data-testid="theme-day">{m['shell.day']()}</button>
				<button type="button" class="pill" aria-pressed={s.theme === 'night'} onclick={() => shell.change({ theme: 'night' })} data-testid="theme-night">{m['shell.night']()}</button>
			</span>
		</div>
		<div class="row">
			<span>{m['shell.ui_size']()}</span>
			<span class="pair">
				<button type="button" class="pill" disabled={s.scale <= SCALE_MIN} onclick={() => shell.change({ scale: s.scale - SCALE_STEP })} aria-label={m['shell.smaller']()}>−</button>
				<span class="value">{percent}</span>
				<button type="button" class="pill" disabled={s.scale >= SCALE_MAX} onclick={() => shell.change({ scale: s.scale + SCALE_STEP })} aria-label={m['shell.bigger']()}>+</button>
			</span>
		</div>
	</Section>
	<Section id="shell-motion" label={m['shell.motion']()} summary={m['shell.this_machine']()}>
		<div class="row"><span>{m['shell.panel_transitions']()}</span><Toggle on={s.panelTransitions} label={m['shell.panel_transitions']()} onchange={(on) => shell.change({ panelTransitions: on })} /></div>
		<div class="row"><span>{m['shell.reduce_motion']()}</span><Toggle on={s.reduceMotion} label={m['shell.reduce_motion']()} onchange={(on) => shell.change({ reduceMotion: on })} /></div>
	</Section>
	<Section id="shell-sound" label={m['shell.sound']()} open={false} summary={m['shell.later']()}>
		<p class="note">{m['shell.sound_later']()}</p>
	</Section>
	<Section id="shell-language" label={m['shell.language']()} summary={getLocale().toUpperCase()}>
		<div class="row">
			<span>{m['shell.interface']()}</span>
			<select value={getLocale()} onchange={(e) => setLocale(e.currentTarget.value as (typeof locales)[number])} aria-label={m['shell.interface']()}>
				{#each locales as l (l)}<option value={l}>{l.toUpperCase()}</option>{/each}
			</select>
		</div>
		<p class="note">{m['shell.text_language_note']()}</p>
	</Section>
	<Section id="shell-folder" label={m['shell.local_folder']()} summary={m['shell.this_machine']()}>
		<div class="row">
			<span class="mono">{folderName ?? m['shell.none']()}</span>
			{#if folder.supported}<button type="button" class="pill" onclick={chooseFolder}>{m['shell.change']()}</button>{/if}
		</div>
		<p class="note">{m['shell.local_folder_note']()}</p>
	</Section>
	<Section id="shell-ai" label={m['shell.ai']()} open={false} summary={m['shell.later']()}>
		<p class="note">{m['shell.ai_note']()} <a href="/docs" target="_blank" rel="noopener">{m['shell.docs']()} ↗</a></p>
	</Section>
	<Section id="shell-notifications" label={m['shell.notifications']()} open={false}>
		<div class="row"><span>{m['shell.push']()}</span><a class="pill" href="/notifications">{m['shell.open']()}</a></div>
	</Section>
	<Section id="shell-layout" label={m['shell.layout']()}>
		<div class="row"><span>{m['shell.reopen_tabs']()}</span><Toggle on={s.reopenTabs} label={m['shell.reopen_tabs']()} onchange={(on) => shell.change({ reopenTabs: on })} /></div>
		<div class="row actions">
			<button type="button" class="pill" onclick={onreset} data-testid="reset-workspace">{m['shell.reset_workspace']()}</button>
			<button type="button" class="pill" onclick={resetAll}>{m['shell.reset_all']()}</button>
		</div>
	</Section>
</div>

<style>
	.settings {
		padding-bottom: var(--editor-pad);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		min-height: var(--editor-row);
		padding: 0 var(--editor-pad);
		text-shadow: var(--editor-halo);
	}
	.row.actions {
		justify-content: flex-start;
	}
	.row select {
		width: auto;
	}
	.pair {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
	}
	.pill {
		display: inline-flex;
		align-items: center;
		padding: 0 0.7rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1rem;
		background: var(--editor-control);
		color: var(--editor-text);
		text-decoration: none;
	}
	.pill[aria-pressed='true'] {
		border-color: var(--editor-primary-line);
		background: var(--editor-primary);
		color: var(--editor-primary-ink);
	}
	.value {
		min-width: 4rem;
		text-align: center;
		font-family: var(--font-mono);
	}
	.mono {
		font-family: var(--font-mono);
	}
	.note {
		margin: 0;
		padding: 0.2rem var(--editor-pad) 0.4rem;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
</style>
