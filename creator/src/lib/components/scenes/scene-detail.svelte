<script lang="ts">
	import type { ContentVersion } from '@scutoons/api';
	import { Chip } from '@scutoons/ui/editor';
	import { fillWords, labelIn, sceneTitle, type PublishOutcome, type SceneRow } from '$lib/editor/scenes.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The open scene in its panel over the preview (lap 2 C5), read only – the scene is edited in the deck: its name, key
	 * and state, what the last Publish answered (the core's problems, or the check not reachable), its facts – where it
	 * plays, the spot, who stands there (a filler, or a placeholder's filter; a spawn zone takes many), its toons – and its
	 * versions. The facts come from the newest version.
	 */
	let {
		row,
		versions,
		published,
		outcome
	}: {
		row: SceneRow;
		versions: ContentVersion[];
		/** the published version's id (null: never published) */
		published: number | null;
		outcome: PublishOutcome | null;
	} = $props();

	const nav = getCreatorNav();
	const f = $derived(row.facts);
	const fill = { of: m['creator.scenes_fill_of'](), below: m['creator.scenes_fill_below']() };
	const when = (iso: string | null | undefined) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : (iso ?? '');
	};
	const newest = $derived(versions[0]?.id ?? null);
	const tags = (id: number) => [id === newest ? m['creator.history_current']() : null, id === published ? m['creator.scenes_published']() : null].filter(Boolean).join(' · ');
</script>

<article class="scene" data-testid="scene-detail">
	<header>
		<h2>{sceneTitle(f)}</h2>
		<span class="state" data-testid="scene-state">
			{#if row.state === 'published'}<Chip label={m['creator.scenes_published']()} on />{:else}<Chip label={m['creator.scenes_draft']()} title={row.state === 'unpublished' ? m['creator.scenes_unpublished']() : undefined} />{/if}
		</span>
		<code class="key">scene:{f.id}</code>
	</header>

	{#if outcome?.kind === 'problems'}
		<div class="problems" role="alert" data-testid="scene-problems">
			<p>{m['creator.scenes_problems']()}</p>
			<ul>{#each outcome.problems as p, i (i)}<li>{p}</li>{/each}</ul>
		</div>
	{:else if outcome?.kind === 'unreachable'}
		<p class="problems" role="alert" data-testid="scene-unreachable">{m['creator.scenes_unreachable']()}</p>
	{:else if outcome?.kind === 'error'}
		<p class="problems" role="alert">{outcome.message}</p>
	{/if}

	<dl class="facts" data-testid="scene-facts">
		<dt>{m['creator.scenes_location']()}</dt>
		<dd>
			{#if f.where?.kind === 'location'}{f.where.ref}
			{:else if f.where?.kind === 'set'}{m['creator.scenes_set']({ set: f.where.set })}
			{:else if f.where}<span class="ph">‹{labelIn(f.where.label, getLocale()) ?? f.where.id}›</span> <span class="muted">{m['creator.scenes_placeholder']({ kinds: f.where.fill.join(' | ') })}{f.where.preview ? ` · ${f.where.preview}` : ''}</span>
			{:else}<span class="muted">{m['creator.scenes_no_location']()}</span>{/if}
		</dd>
		{#if f.spot}
			<dt>{m['creator.scenes_spot']()}</dt>
			<dd>{f.spot}</dd>
		{/if}
		<dt>{m['creator.scenes_who']()}</dt>
		<dd>
			{#if f.holders.length}
				<ul class="holders">
					{#each f.holders as h (h.id)}
						<li data-testid="scene-holder-{h.id}">
							{#if h.ref}{h.ref}{:else}<span class="ph">‹{labelIn(h.label, getLocale()) ?? h.id}›</span> <span class="muted">{h.fill ? fillWords(h.fill, fill) : ''}</span>{/if}
							<span class="muted">
								{#if h.many} · {m['creator.scenes_many']({ arrange: h.many.arrange })}{#if h.many.max != null}, {m['creator.scenes_max']({ max: h.many.max })}{/if}{/if}
								{#if h.spot} · {h.spot}{/if}
								{#if h.enter === 'join'} · {m['creator.scenes_join']()}{/if}
								{#if h.rig} · {m['creator.scenes_rig']({ rig: h.rig })}{/if}
							</span>
						</li>
					{/each}
				</ul>
			{:else}<span class="muted">{m['creator.scenes_nobody']()}</span>{/if}
		</dd>
		<dt>{m['creator.scenes_toons']()}</dt>
		<dd>
			{#if f.toons.length}{f.toons.map((t) => nav.toonName(t)).join(' · ')}{:else}<span class="muted">{m['creator.scenes_toons_none']()}</span>{/if}
		</dd>
	</dl>

	<h3>{m['creator.scenes_versions']()}</h3>
	<ol class="versions" data-testid="scene-versions">
		{#each versions as v (v.id)}
			{@const tag = tags(v.id)}
			<li>
				<span>{when(v.created_at)}</span>
				{#if tag}<Chip label={tag} on={v.id === published} />{/if}
				<br /><span class="muted">{v.author_email ? m['creator.history_by']({ author: v.author_email }) : m['creator.history_import']()}</span>
				{#if v.message}<br /><i>{v.message}</i>{/if}
			</li>
		{/each}
	</ol>
</article>

<style>
	.scene {
		display: flex;
		flex-direction: column;
		gap: var(--editor-gap);
		padding: var(--editor-pad);
		font-size: var(--editor-small);
	}
	header {
		display: grid;
		grid-template-columns: 1fr auto;
		align-items: center;
		gap: 0.1rem var(--editor-gap);
	}
	h2 {
		margin: 0;
		font-family: var(--editor-sans);
		font-size: var(--editor-title, 1.15rem);
		font-weight: 400;
		letter-spacing: normal;
		text-transform: none;
	}
	h3 {
		margin: 0.5rem 0 0;
		padding-bottom: 0.3rem;
		border-bottom: var(--editor-edge) solid var(--tool-line);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--tool-muted);
	}
	.key {
		grid-column: 1 / -1;
		font-family: var(--editor-mono, ui-monospace, monospace);
		font-size: var(--editor-label);
	}
	.key,
	.muted {
		color: var(--tool-muted);
	}
	.problems {
		margin: 0;
		padding: 0.5rem 0.75rem;
		border: var(--editor-edge) solid var(--tool-stop);
		border-radius: var(--editor-radius);
		color: var(--tool-stop);
	}
	.problems p {
		margin: 0;
	}
	.problems ul {
		margin: 0.25rem 0 0;
		padding-left: 1.25rem;
	}
	.facts {
		display: grid;
		grid-template-columns: minmax(5rem, max-content) 1fr;
		gap: 0.5rem 1.25rem;
		margin: 0;
	}
	dt {
		font-size: var(--editor-label);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--tool-muted);
		padding-top: 0.1rem;
	}
	dd {
		margin: 0;
		min-width: 0;
	}
	.holders {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.versions {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.versions li > :global(.kit-chip) {
		margin-left: 0.3rem;
		vertical-align: middle;
	}
</style>
