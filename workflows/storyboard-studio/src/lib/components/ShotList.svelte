<script>
	// The chapter's shots in the toon's order, by scene (owner 2026-10-19: "the left side is our timeline vertically, the
	// toon order so to speak"): the frame, the id, view + format, the review (ok / change), the number of variants – the
	// same in every step. A click = only that shot, Ctrl / ⌘ + click = in or out of the picked set, Shift + click = the
	// range from the last clicked one (studio.svelte.js clickShot) – the timeline shows the picked ones. The current shot
	// (the last clicked, the one the panels edit) = sc-selected (tools.css); a picked one = an ink frame with a bar, quieter.
	// Unsaved changes → an in-page confirmation first (pickShot).
	import { ch, clickShot, pickedShots, studio } from '../studio.svelte.js';

	const stamp = $derived(studio.version);
	const picked = $derived(new Set(pickedShots()));
	const HOW = 'Click: only this shot · Ctrl + click: add to / take off the timeline · Shift + click: the shots up to here';
	const click = (e, id) => clickShot(id, { toggle: e.ctrlKey || e.metaKey, range: e.shiftKey });
	/** the card's facts: format + aspect, view, variants, the handheld takes */
	const facts = (shot) => [`${shot.format} ${shot.aspect}`, shot.view ?? '–', studio.data.variants[shot.id] && `${studio.data.variants[shot.id]} var.`, studio.data.handheld?.[shot.id] != null && `handheld ${studio.data.handheld[shot.id]}`].filter(Boolean);
</script>

<nav class="shots" aria-label="Shots" data-sc-region>
	{#each studio.data?.scenes ?? [] as sc (sc.file)}
		<h2>{sc.sb.source?.scene} {sc.sb.source?.heading ?? ''}</h2>
		<ul>
			{#each sc.sb.groups ?? [] as g (g.label)}
				{#each g.shots ?? [] as shot (shot.id)}
					<li>
						<button type="button" class="shot" class:sc-selected={studio.sel?.id === shot.id} class:picked={picked.has(shot.id)} aria-current={studio.sel?.id === shot.id ? 'true' : undefined} aria-pressed={picked.has(shot.id)} data-control-exempt="a card" onclick={(e) => click(e, shot.id)} title={[shot.action?.de, HOW].filter(Boolean).join(' – ')}>
							<img src={`/out/${ch()}/frames/${shot.id}.png?v=${stamp}`} alt="" loading="lazy" />
							<span class="text">
								<span class="id"><span class="dot {shot.review?.status ?? ''}" title={shot.review ? `${shot.review.status} ${shot.review.date ?? ''}: ${shot.review.note ?? ''}` : 'no review yet'}></span>{shot.id}</span>
								<span class="meta"><span class="facts">{#each facts(shot) as f, i (i)}<span>{f}</span>{/each}</span></span>
								<span class="action">{shot.action?.de ?? ''}</span>
							</span>
						</button>
					</li>
				{/each}
			{/each}
		</ul>
	{/each}
</nav>

<style>
	.shots {
		height: 100%;
		overflow: auto;
		padding: var(--space-1);
		box-sizing: border-box;
	}
	h2 {
		margin: var(--space-1) 0 var(--space-05);
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	h2:first-child {
		margin-top: 0;
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
		/* a card, not a button row: the frame as tall as the text (buttons.css centres) */
		align-items: stretch;
		justify-content: stretch;
		gap: var(--space-05);
		width: 100%;
		padding: var(--space-025);
		text-align: left;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid transparent;
		cursor: pointer;
		/* Shift + click picks a range – it never selects the cards' text */
		user-select: none;
		height: auto;
		min-height: 0;
	}
	.shot:hover {
		border-color: var(--ink);
	}
	/* picked for the timeline, not the current one: an ink frame with a bar on its left – quieter than the inversion */
	.shot.picked:not(:global(.sc-selected)) {
		border-color: var(--ink);
		box-shadow: inset var(--border-width-focus) 0 0 var(--ink);
	}
	img {
		width: 3rem;
		max-height: 5.5rem;
		object-fit: cover;
		object-position: top;
		background: var(--ink);
		display: block;
	}
	.text {
		display: grid;
		gap: 0.125rem;
		min-width: 0;
	}
	.id,
	.meta,
	.action {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	/* the id on one line (the full text in the panel) */
	.id {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* the facts wrap between themselves, never inside one ("vertical 1:3 · r…" at the 15rem list); the "·" before each
	   hangs left, so the one that starts a line falls outside the clip */
	.meta {
		overflow: hidden;
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		margin-left: -1.2em;
	}
	.facts > span {
		white-space: nowrap;
	}
	.facts > span::before {
		content: '·';
		display: inline-block;
		width: 1.2em;
		text-align: center;
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
	}
	.dot.ok {
		background: var(--badge-published);
	}
	.dot.change {
		background: var(--badge-warning);
	}
</style>
