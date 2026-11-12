<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { ContentVersion } from '@scutoons/api';
	import { fillWords, labelIn, sceneTitle, type PublishOutcome, type SceneRow } from '$lib/editor/scenes.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One scene's page (#490, draft 3): its id and state, what the last Publish answered (the core's problems, or the
	 * check not reachable), its facts – where it plays, the spot, who stands there (a filler, or a placeholder's filter;
	 * a spawn zone takes many), its toons – and its versions (restore). Read only: the facts come from the newest version.
	 */
	let {
		row,
		versions,
		published,
		outcome,
		restore,
		status
	}: {
		row: SceneRow;
		versions: ContentVersion[];
		/** the published version's id (null: never published) */
		published: number | null;
		outcome: PublishOutcome | null;
		restore: (v: ContentVersion) => void;
		/** the state badge */
		status: Snippet;
	} = $props();

	const nav = getCreatorNav();
	const f = $derived(row.facts);
	const fill = { of: m['creator.scenes_fill_of'](), below: m['creator.scenes_fill_below']() };
	const when = (iso: string | null | undefined) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(getLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(d) : (iso ?? '');
	};
	const newest = $derived(versions[0]?.id ?? null);
</script>

<article class="scene" data-testid="scene-detail">
	<header>
		<h1>{sceneTitle(f)}</h1>
		<code class="key">scene:{f.id}</code>
		<span class="state" data-testid="scene-state">{@render status()}</span>
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
	{:else if row.state === 'published'}
		<p class="sc-empty">{m['creator.scenes_nothing_new']()}</p>
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

	<h2>{m['creator.scenes_versions']()}</h2>
	<ol class="versions" data-testid="scene-versions">
		{#each versions as v (v.id)}
			<li class:live={v.id === published}>
				<span class="what">
					<span>{when(v.created_at)}</span>
					{#if v.id === newest}<span class="tag">{m['creator.history_current']()}</span>{/if}
					{#if v.id === published}<span class="tag">{m['creator.scenes_published']()}</span>{/if}
					<br /><span class="muted">{v.author_email ? m['creator.history_by']({ author: v.author_email }) : m['creator.history_import']()}</span>
					{#if v.message}<br /><i>{v.message}</i>{/if}
				</span>
				{#if v.id !== newest}
					<button type="button" class="ghost" onclick={() => restore(v)} data-testid="scene-restore-{v.id}">{m['creator.restore']()}</button>
				{/if}
			</li>
		{/each}
	</ol>
</article>

<style>
	.scene {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.75rem;
	}
	h1 {
		margin: 0;
	}
	h2 {
		margin: 0.5rem 0 0;
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
	}
	.key,
	.muted {
		color: var(--text-muted);
	}
	.state {
		margin-left: auto;
	}
	.problems {
		margin: 0;
		padding: 0.5rem 0.75rem;
		border: var(--border-width-regular) solid var(--state-error);
		color: var(--state-error);
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
		grid-template-columns: minmax(6rem, max-content) 1fr;
		gap: 0.5rem 1.5rem;
		margin: 0;
	}
	dt {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		padding-top: 0.15rem;
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
	}
	.versions li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.4rem 0;
		border-top: var(--border-width-regular) solid var(--border-subtle);
	}
	.versions li.live {
		border-top-color: var(--ink);
	}
	.tag {
		margin-left: 0.4rem;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	@media (max-width: 767px) {
		.facts {
			grid-template-columns: 1fr;
			gap: 0.15rem;
		}
		dd {
			margin-bottom: 0.5rem;
		}
	}
</style>
