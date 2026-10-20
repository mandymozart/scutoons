<script lang="ts">
	// The chapter's shots in the toon's order, by scene – the page's own list in the sidebar (owner 2026-10-19: "the left
	// side is our timeline vertically, the toon order so to speak"; #92 "the page's own list beside the menu"): the
	// schematic as the frame (@scutoons/content shotSvg – the 3D view's frames come with #124), the id, format + view,
	// the review dot. A click = only that shot, Ctrl / ⌘ + click = in or out of the picked set, Shift + click = the range
	// from the last clicked one (storyboard-shots.ts pickShot). The current shot (the one the panels edit) is
	// sc-selected; a picked one an ink frame with a bar, quieter.
	import { region } from '@scutoons/ui';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { shotFacts, svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const picked = $derived(new Set(sb.picked));
	/** the scenes with their shots, in order (a scene without shots shows only its heading) */
	const scenes = $derived(
		(sb.data?.scenes ?? []).map((scene, si) => ({ scene, si, shots: sb.entries.filter((e) => e.sceneIndex === si) }))
	);
	/** the frames, made again when the document changes */
	const frames = $derived(new Map(sb.entries.map((e) => [e.id, svgSource(shotSvg(e.shot, e.scene, { noText: true }).svg)])));
	const reviewWord = (status: string | undefined) => (status === 'ok' ? m['storyboard.review_ok']() : status === 'change' ? m['storyboard.review_change']() : m['storyboard.review_none']());
</script>

<nav class="shots" aria-label={m['storyboard.panel_shot']()} use:region={'storyboard-shots'} data-testid="shot-list">
	{#each scenes as { scene, si, shots } (si)}
		<p class="sc-nav-head">{scene.source?.scene ?? ''} {scene.source?.heading ?? ''}</p>
		<ul>
			{#each shots as e (e.id)}
				<li>
					<button
						type="button"
						class="shot"
						class:sc-selected={sb.current === e.id}
						class:picked={picked.has(e.id) && sb.current !== e.id}
						aria-current={sb.current === e.id ? 'true' : undefined}
						aria-pressed={picked.has(e.id)}
						data-control-exempt="a card"
						data-drawer-pick
						data-testid="shot-{e.id}"
						title={[e.shot.action?.de, m['storyboard.list_hint']()].filter(Boolean).join(' – ')}
						onclick={(ev) => sb.pick(e.id, ev)}
					>
						<img src={frames.get(e.id)} alt="" />
						<span class="text">
							<span class="id"><span class="dot {e.shot.review?.status ?? ''}" title={[reviewWord(e.shot.review?.status), e.shot.review?.note].filter(Boolean).join(': ')}></span>{e.id}</span>
							<span class="facts">{#each shotFacts(e.shot) as f, i (i)}<span>{f}</span>{/each}</span>
							<span class="action">{e.shot.action?.de ?? ''}</span>
						</span>
					</button>
				</li>
			{/each}
		</ul>
	{/each}
</nav>

<style>
	.shots {
		padding: 0 var(--space-1) var(--space-1);
		box-sizing: border-box;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-025);
	}
	.shot {
		display: grid;
		grid-template-columns: 3rem minmax(0, 1fr);
		/* a card, not a button row: the frame as tall as the text (the creator's buttons centre) */
		align-items: stretch;
		justify-content: stretch;
		gap: var(--space-05);
		width: 100%;
		height: auto;
		min-height: 0;
		padding: var(--space-025);
		text-align: left;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid transparent;
		cursor: pointer;
		/* Shift + click picks a range – it never selects the cards' text */
		user-select: none;
	}
	.shot:hover {
		border-color: var(--ink);
	}
	/* picked, not the current one: an ink frame with a bar on its left – quieter than the inversion */
	.shot.picked {
		border-color: var(--ink);
		box-shadow: inset var(--border-width-focus) 0 0 var(--ink);
	}
	img {
		display: block;
		width: 3rem;
		max-height: 5.5rem;
		object-fit: cover;
		object-position: top;
		background: var(--paper);
		border: var(--border-width-hairline) solid var(--border-subtle);
		box-sizing: border-box;
	}
	.text {
		display: grid;
		gap: 0.125rem;
		min-width: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.id {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* the facts wrap between themselves, never inside one */
	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-05);
	}
	.facts > span {
		white-space: nowrap;
	}
	.action {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.dot {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		margin-right: var(--space-025);
		border: var(--border-width-regular) solid currentColor;
		border-radius: 50%;
		box-sizing: border-box;
	}
	.dot.ok {
		background: currentColor;
	}
	.dot.change {
		background: var(--state-warning);
	}
</style>
