<script lang="ts">
	// The storyboard's rhythm (STORYBOARD.md §6, @scutoons/content/storyboard summarize / measureShot / TARGET): the
	// chapter's reading length against the target, each scene's, and the current shot's. The storyboard check's errors
	// and warnings are the creator's Problems panel (the ActionBar) – a row there picks the shot it names.
	import { measureShot, summarize, TARGET } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const summary = $derived(summarize((sb.data?.scenes ?? []).map((scene) => ({ sb: scene }))));
	const shot = $derived(sb.sel ? measureShot(sb.sel.shot) : null);
	const minutes = (seconds: number) => (seconds / 60).toFixed(1);
	const screens = (n: number) => n.toFixed(1);
	const line = (s: { shots: number; screens: number; seconds: number; words: number }) =>
		m['storyboard.rhythm_line']({ shots: s.shots, screens: screens(s.screens), minutes: minutes(s.seconds), words: s.words });
	const outside = $derived(summary.total.seconds / 60 < TARGET.minutes[0] || summary.total.seconds / 60 > TARGET.minutes[1]);
</script>

<div class="panel" data-testid="rhythm-panel">
	<h3>{m['storyboard.rhythm_chapter']()}</h3>
	<p class:outside data-testid="rhythm-total">{line(summary.total)}</p>
	<p class="muted">{m['storyboard.rhythm_target']({ screens: TARGET.screens.join('–'), minutes: TARGET.minutes.join('–') })}</p>
	{#if shot && sb.sel}
		<h3>{m['storyboard.rhythm_shot']()} · {sb.sel.id}</h3>
		<p>{m['storyboard.rhythm_shot_line']({ screens: screens(shot.screens), seconds: shot.seconds.toFixed(0), words: shot.words })}</p>
	{/if}
	<ul>
		{#each summary.scenes as s, i (i)}
			<li class:current={sb.sel?.sceneIndex === i}>
				<span class="scene">{s.scene ?? ''} {s.heading ?? ''}</span>
				<span>{line(s)}</span>
			</li>
		{/each}
	</ul>
</div>

<style>
	.panel {
		display: grid;
		gap: var(--space-05);
		padding: var(--space-1);
		box-sizing: border-box;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	h3 {
		margin: var(--space-05) 0 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	h3:first-child {
		margin-top: 0;
	}
	p {
		margin: 0;
	}
	.muted {
		color: var(--text-muted);
	}
	.outside {
		color: var(--state-warning);
	}
	ul {
		display: grid;
		gap: var(--space-05);
		margin: var(--space-05) 0 0;
		padding: var(--space-05) 0 0;
		list-style: none;
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	li {
		display: grid;
		padding-left: var(--space-05);
		border-left: var(--border-width-focus) solid transparent;
	}
	li.current {
		border-left-color: var(--ink);
	}
	.scene {
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
</style>
