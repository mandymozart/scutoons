<script>
	// The storyboard studio (owner 2026-10-09: "use our scutoons ui and design dogmas"): one AppShell like the tools –
	// the chapter's shots as the sidebar (resizable, folds to a rail), the 3D view as the page, the shot's panels in the
	// right dock (Shot · Script · Variants · Luma; each folds to its rail, drags wider). agents/DOCKING.md, DESIGN.md.
	import { onMount } from 'svelte';
	import { AppShell, DialogHost, Icon, installTooltips, panelWidth, useDock } from '@scutoons/ui';
	import { loadChapter, loadChapters, run, studio } from './lib/studio.svelte.js';
	import ShotList from './lib/components/ShotList.svelte';
	import Viewport from './lib/components/Viewport.svelte';
	import ShotPanel from './lib/components/ShotPanel.svelte';
	import ScriptPanel from './lib/components/ScriptPanel.svelte';
	import VariantsPanel from './lib/components/VariantsPanel.svelte';
	import PromptPanel from './lib/components/PromptPanel.svelte';

	const width = (kind, rem) => panelWidth(`storyboard-studio:${kind}`, rem, undefined, { foldable: true });
	const sizes = { shot: width('shot', 24), script: width('script', 20), variants: width('variants', 16), prompt: width('prompt', 26) };
	const PANELS = [
		{ kind: 'shot', title: 'Shot', icon: 'sliders' },
		{ kind: 'script', title: 'Script', icon: 'file-text' },
		{ kind: 'variants', title: 'Variants', icon: 'layers' },
		{ kind: 'prompt', title: 'Luma', icon: 'send' }
	];
	let open = $state({ shot: true, script: true, variants: false, prompt: false });
	const docked = (kind, panel) => (open[kind] && studio.sel ? { kind: `storyboard-${kind}`, size: sizes[kind], title: PANELS.find((p) => p.kind === kind).title, icon: PANELS.find((p) => p.kind === kind).icon, close: () => (open[kind] = false), panel } : null);
	useDock(() => docked('shot', shotPanel));
	useDock(() => docked('script', scriptPanel));
	useDock(() => docked('variants', variantsPanel));
	useDock(() => docked('prompt', promptPanel));

	onMount(() => {
		installTooltips();
		loadChapters().catch((e) => (studio.message = e.message));
	});

	const summary = $derived.by(() => {
		const t = studio.data?.summary?.total;
		return t ? `${t.shots} shots · ${t.screens.toFixed(1)} screens · ${(t.seconds / 60).toFixed(1)} min · ${studio.data.problems.length} notes` : '';
	});
	const ACTIONS = [
		{ cmd: 'validate', label: 'Check', title: 'Run the storyboard check (validate.mjs)' },
		{ cmd: 'frames', label: 'All frames', title: 'Render every frame of the chapter (~4 min)' },
		{ cmd: 'refpacks', label: 'Reference packs', title: 'One folder per shot for Luma (render, style, characters, prompt)' },
		{ cmd: 'compile', label: 'Chapter draft', title: 'The chapter draft for the creator (compile.mjs) – nothing is saved' }
	];
</script>

{#snippet shotPanel()}<ShotPanel />{/snippet}
{#snippet scriptPanel()}<ScriptPanel />{/snippet}
{#snippet variantsPanel()}<VariantsPanel />{/snippet}
{#snippet promptPanel()}<PromptPanel />{/snippet}

<div class="studio sc-tool">
	<AppShell mode="fill" resizeKey="storyboard-studio:shots" foldable railLabel="Shots" sidebarLabel="Shots" sidebarTag="aside" dockLabel="Shot panels" pageLabel="3D view">
		{#snippet topbar()}
			<header class="topbar">
				<span class="brand"><img src="/scutoons-logo.svg" alt="" onerror={(e) => e.currentTarget.remove()} />Storyboard Studio</span>
				<select aria-label="Chapter" bind:value={studio.chapter} onchange={() => loadChapter()}>
					{#each studio.chapters as c (`${c.toon}/${c.ch}`)}
						<option value={`${c.toon}/${c.ch}`}>{c.toon} · {c.ch}</option>
					{/each}
				</select>
				<span class="summary">{summary}</span>
				<span class="grow"></span>
				<div class="sc-seg" role="group" aria-label="Panels">
					{#each PANELS as p (p.kind)}
						<button type="button" class:sc-selected={open[p.kind]} aria-pressed={open[p.kind]} title={`${p.title} panel`} onclick={() => (open[p.kind] = !open[p.kind])}><Icon name={p.icon} /> {p.title}</button>
					{/each}
				</div>
				<div class="sc-seg" role="group" aria-label="Scripts">
					{#each ACTIONS as a (a.cmd)}
						<button type="button" title={a.title} onclick={() => run(a.cmd)}>{a.label}</button>
					{/each}
				</div>
			</header>
		{/snippet}
		{#snippet sidebar()}
			<ShotList />
		{/snippet}
		<Viewport />
	</AppShell>
	{#if studio.message}
		<p class="status" role="status">{studio.message}</p>
	{/if}
	<DialogHost />
</div>

<style>
	.studio {
		height: 100dvh;
	}
	.topbar {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		height: var(--tool-bar-height);
		padding: 0 var(--space-1);
		box-sizing: border-box;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
		overflow-x: auto;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
	}
	.brand img {
		height: 1.25rem;
		width: auto;
	}
	.summary {
		color: var(--ink-muted, var(--ink));
		font-size: var(--type-caption-size);
		white-space: nowrap;
	}
	.grow {
		flex: 1;
	}
	.sc-seg button {
		white-space: nowrap;
	}
	.status {
		position: fixed;
		left: var(--space-1);
		bottom: var(--space-1);
		margin: 0;
		max-width: min(40rem, 90vw);
		padding: var(--space-05) var(--space-1);
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
		z-index: 10;
	}
</style>
