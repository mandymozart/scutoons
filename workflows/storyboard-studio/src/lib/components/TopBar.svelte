<script>
	// The studio's top bar (research/STUDIO-LAYOUT.md concept B, owner 2026-10-19): the chapter, the six steps as a square
	// tab row (the current one inverted; keys 1–6), the panels of the step as an icon group (toggling changes this step's
	// set only), and the Chapter menu with the scripts (Check, All frames, Reference packs, Chapter draft) and the taste
	// rule – so the bar never overflows at 1440. Save / review live in the ActionBar (StudioActions).
	import { Icon, Menu } from '@scutoons/ui';
	import { fail, loadChapter, run, studio } from '../studio.svelte.js';
	import { addRule } from '../actions.svelte.js';
	import { isOpen, PANELS, setOpen, setStep, STEPS, steps } from '../steps.svelte.js';

	const SCRIPTS = [
		{ cmd: 'validate', label: 'Check', title: 'Run the storyboard check (validate.mjs)' },
		{ cmd: 'frames', label: 'All frames', title: 'Render every frame of the chapter (~4 min)' },
		{ cmd: 'refpacks', label: 'Reference packs', title: 'One folder per shot for Luma (render, style, characters, prompt)' },
		{ cmd: 'compile', label: 'Chapter draft', title: 'The chapter draft for the creator (compile.mjs) – nothing is saved' }
	];
	const pick = () => loadChapter().catch(fail);
</script>

<header class="topbar">
	<span class="brand"><img src="/scutoons-logo.svg" alt="" onerror={(e) => e.currentTarget.remove()} />Storyboard Studio</span>
	<select aria-label="Chapter" title="Chapter" bind:value={studio.chapter} onchange={pick}>
		{#each studio.chapters as c (`${c.toon}/${c.ch}`)}
			<option value={`${c.toon}/${c.ch}`} selected={`${c.toon}/${c.ch}` === studio.chapter}>{c.toon} · {c.ch}</option>
		{/each}
	</select>
	<div class="steps sc-seg" role="group" aria-label="Steps">
		{#each STEPS as s, i (s.id)}
			<button type="button" aria-pressed={steps.current === s.id} title="{s.title} ({i + 1})" onclick={() => setStep(s.id)} data-testid="step-{s.id}"><kbd>{i + 1}</kbd>{s.label}</button>
		{/each}
	</div>
	<span class="grow"></span>
	<div class="sc-seg" role="group" aria-label="Panels of this step">
		{#each PANELS as p (p.kind)}
			<button type="button" class="is-square" aria-pressed={isOpen(p.kind)} aria-label={p.title} title="{p.title} panel{p.kind === 'shot' ? ' (N folds the step\'s panels)' : ''}" onclick={() => setOpen(p.kind, !isOpen(p.kind))} data-testid="panel-{p.kind}"><Icon name={p.icon} /></button>
		{/each}
	</div>
	<Menu label="Chapter" align="end">
		{#snippet trigger()}Chapter <Icon name="chevron-down" />{/snippet}
		{#each SCRIPTS as a (a.cmd)}
			<button type="button" title={a.title} onclick={() => run(a.cmd).catch(fail)}>{a.label}</button>
		{/each}
		<button type="button" title="A rule for storyboard-taste.md – the drafting agents read it" onclick={addRule}>Add a taste rule …</button>
	</Menu>
</header>

<style>
	.topbar {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		height: var(--tool-bar-height);
		padding: 0 var(--space-1);
		box-sizing: border-box;
		background: var(--paper);
		border-bottom: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.brand img {
		height: 1.25rem;
		width: auto;
	}
	.grow {
		flex: 1;
	}
	.steps kbd {
		margin-right: var(--space-05);
		font: inherit;
		opacity: 0.6;
	}
	/* narrower windows: the step's number goes, the word stays */
	@media (max-width: 1279px) {
		.steps kbd,
		.brand {
			display: none;
		}
	}
</style>
