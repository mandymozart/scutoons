<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Chip, IconButton, PillTabs, Section, Select, Toggle } from '@scutoons/ui/editor';
	import { craftName } from '@scutoons/api';
	import { crafts } from '$lib/shell/crafts.svelte.ts';
	import { toaster } from '$lib/toasts.ts';
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
	const mine = $derived(crafts.mine?.crafts ?? []);
	async function toggleCraft(id: string) {
		const next = crafts.list.map((c) => c.id).filter((c) => (c === id ? !mine.includes(id) : mine.includes(c)));
		try {
			await crafts.save(next);
		} catch (e) {
			toaster.addToast({ message: e instanceof Error ? e.message : String(e), type: 'error' });
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
			<PillTabs label={m['shell.theme']()} on={s.theme} onselect={(id) => shell.change({ theme: id as 'day' | 'night' })} items={[{ id: 'day', label: m['shell.day'](), testid: 'theme-day' }, { id: 'night', label: m['shell.night'](), testid: 'theme-night' }]} />
		</div>
		<div class="row" title={m['shell.colorblind_hint']()}>
			<span>{m['shell.colorblind']()}</span>
			<Toggle on={s.colorblind} label={m['shell.colorblind']()} title={m['shell.colorblind_hint']()} onchange={(on) => shell.change({ colorblind: on })} />
		</div>
		<div class="row">
			<span>{m['shell.ui_size']()}</span>
			<span class="pair">
				<IconButton label={m['shell.smaller']()} icon="minus" disabled={s.scale <= SCALE_MIN} onclick={() => shell.change({ scale: s.scale - SCALE_STEP })} />
				<span class="value">{percent}</span>
				<IconButton label={m['shell.bigger']()} icon="plus" disabled={s.scale >= SCALE_MAX} onclick={() => shell.change({ scale: s.scale + SCALE_STEP })} />
			</span>
		</div>
	</Section>
	{#if crafts.list.length}
		<Section id="shell-crafts" label={m['shell.crafts']()} summary={m['shell.crafts_summary']({ count: mine.length })}>
			<div class="crafts" data-testid="settings-crafts">
				{#each crafts.list as craft (craft.id)}
					<Chip label={craftName(craft, getLocale())} icon={craft.icon} on={mine.includes(craft.id)} disabled={!crafts.mine?.can_edit} title={crafts.mine?.can_edit ? undefined : m['shell.crafts_admin']()} onclick={() => toggleCraft(craft.id)} />
				{/each}
			</div>
		</Section>
	{/if}
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
			<Select value={getLocale()} options={locales.map((l) => ({ value: l, label: l.toUpperCase() }))} label={m['shell.interface']()} onchange={(v) => setLocale(v as (typeof locales)[number])} />
		</div>
	</Section>
	<Section id="shell-folder" label={m['shell.local_folder']()} summary={m['shell.this_machine']()}>
		<div class="row">
			<span>{folderName ?? m['shell.none']()}</span>
			{#if folder.supported}<Button onclick={chooseFolder}>{m['shell.change']()}</Button>{/if}
		</div>
		<p class="note">{m['shell.local_folder_note']()}</p>
	</Section>
	<Section id="shell-ai" label={m['shell.ai']()} open={false} summary={m['shell.later']()}>
		<p class="note">{m['shell.ai_note']()} <a href="/docs" target="_blank" rel="noopener">{m['shell.docs']()} ↗</a></p>
	</Section>
	<Section id="shell-notifications" label={m['shell.notifications']()} open={false}>
		<div class="row"><span>{m['shell.push']()}</span><Button href="/notifications">{m['shell.open']()}</Button></div>
	</Section>
	<Section id="shell-layout" label={m['shell.layout']()}>
		<div class="row"><span>{m['shell.reopen_tabs']()}</span><Toggle on={s.reopenTabs} label={m['shell.reopen_tabs']()} onchange={(on) => shell.change({ reopenTabs: on })} /></div>
		<div class="row"><span>{m['shell.gizmo_always']()}</span><Toggle on={s.gizmoAlways} label={m['shell.gizmo_always']()} onchange={(on) => shell.change({ gizmoAlways: on })} /></div>
		<div class="row actions">
			<Button onclick={onreset} data-testid="reset-workspace">{m['shell.reset_workspace']()}</Button>
			<Button onclick={resetAll}>{m['shell.reset_all']()}</Button>
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
		text-shadow: var(--tool-halo);
	}
	.crafts {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		padding: 0.2rem var(--editor-pad) 0.4rem;
	}
	.row.actions {
		justify-content: flex-start;
	}
	.pair {
		display: inline-flex;
		align-items: center;
		gap: 0.15rem;
	}
	.value {
		min-width: 4rem;
		text-align: center;
	}
	.note {
		margin: 0;
		padding: 0.2rem var(--editor-pad) 0.4rem;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
</style>
