<script>
	// The chapter's shots in reading order, by scene: the frame, the id, view + format, the review (ok / change), the
	// number of variants. Selected = sc-selected (tools.css). Unsaved changes → an in-page confirmation first.
	import { confirmDialog } from '@scutoons/ui';
	import { ch, select, studio } from '../studio.svelte.js';

	const stamp = $derived(studio.version);
	async function pick(id) {
		if (studio.dirty && id !== studio.sel?.id) {
			const ok = await confirmDialog({ title: 'Discard the changes?', message: `${studio.sel.id} has unsaved changes.`, confirmLabel: 'Discard' });
			if (!ok) return;
		}
		select(id);
	}
</script>

<nav class="shots" aria-label="Shots" data-sc-region>
	{#each studio.data?.scenes ?? [] as sc (sc.file)}
		<h2>{sc.sb.source?.scene} {sc.sb.source?.heading ?? ''}</h2>
		<ul>
			{#each sc.sb.groups ?? [] as g (g.label)}
				{#each g.shots ?? [] as shot (shot.id)}
					<li>
						<button type="button" class="shot" class:sc-selected={studio.sel?.id === shot.id} data-control-exempt="a card" onclick={() => pick(shot.id)} title={shot.action?.de ?? ''}>
							<img src={`/out/${ch()}/frames/${shot.id}.png?v=${stamp}`} alt="" loading="lazy" />
							<span class="text">
								<span class="id"><span class="dot {shot.review?.status ?? ''}" title={shot.review ? `${shot.review.status} ${shot.review.date ?? ''}: ${shot.review.note ?? ''}` : 'no review yet'}></span>{shot.id}</span>
								<span class="meta">{shot.format} {shot.aspect} · {shot.view ?? '–'}{studio.data.variants[shot.id] ? ` · ${studio.data.variants[shot.id]} var.` : ''}{studio.data.handheld?.[shot.id] != null ? ` · handheld ${studio.data.handheld[shot.id]}` : ''}</span>
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
		gap: var(--space-05);
		width: 100%;
		padding: var(--space-025);
		text-align: left;
		background: var(--paper);
		border: var(--border-width-regular) solid transparent;
		cursor: pointer;
		height: auto;
		min-height: 0;
	}
	.shot:hover {
		border-color: var(--ink);
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
	/* id and meta on one line each (the full text in the card's tooltip / the panel) */
	.id,
	.meta {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
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
		background: var(--state-success);
	}
	.dot.change {
		background: var(--state-warning);
	}
</style>
