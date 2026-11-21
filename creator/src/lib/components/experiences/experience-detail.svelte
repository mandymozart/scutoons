<script lang="ts">
	import type { Snippet } from 'svelte';
	import { crossAppClick } from '@scutoons/pwa';
	import { Section } from '@scutoons/ui/editor';
	import type { ContentVersion } from '$lib/api.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { experienceFacts, readerHref, titleIn, type ExperienceRow } from '$lib/editor/experiences.ts';
	import type { PublishOutcome } from '$lib/editor/scenes.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ExperienceOutline from '$lib/components/experiences/experience-outline.svelte';

	/** One experience, read only: what Publish answered, its facts, its contents and its versions. */
	let {
		row,
		content,
		versions,
		published,
		outcome,
		typeName,
		toons,
		status
	}: {
		row: ExperienceRow;
		/** the newest version's document */
		content: unknown;
		versions: ContentVersion[];
		/** the published version's id (null: never published) */
		published: number | null;
		outcome: PublishOutcome | null;
		typeName: string;
		/** the toons' names, joined */
		toons: string;
		status: Snippet;
	} = $props();

	const locale = getLocale();
	const f = $derived(experienceFacts(content, row.id));
	const when = (iso: string | null | undefined) => {
		const d = iso ? new Date(iso) : null;
		return d && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(d) : (iso ?? '');
	};
	const newest = $derived(versions[0]?.id ?? null);
</script>

<article class="xr" data-testid="experience-detail">
	<header>
		<h1>{titleIn(f.title, locale, f.id)}</h1>
		<code class="muted">experience:{f.id}</code>
		<span class="end">
			<a class="reader" href={readerHref(VIEWER_URL, f.id)} target="_blank" rel="noopener" onclick={crossAppClick} data-testid="experience-reader">{m['creator.experiences_open_reader']()} ↗</a>
			<span data-testid="experience-state">{@render status()}</span>
		</span>
	</header>

	{#if outcome?.kind === 'problems'}
		<div class="problems" role="alert" data-testid="experience-problems">
			<p>{m['creator.experiences_problems']()}</p>
			<ul>{#each outcome.problems as p, i (i)}<li>{p}</li>{/each}</ul>
		</div>
	{:else if outcome?.kind === 'unreachable'}
		<p class="problems" role="alert">{m['creator.experiences_unreachable']()}</p>
	{:else if outcome?.kind === 'error'}
		<p class="problems" role="alert">{outcome.message}</p>
	{:else if row.state === 'published'}
		<p class="sc-empty">{m['creator.experiences_nothing_new']()}</p>
	{/if}

	<dl class="facts" data-testid="experience-facts">
		<dt>{m['creator.experiences_type']()}</dt>
		<dd>{typeName}</dd>
		<dt>{m['creator.experiences_title']()}</dt>
		<dd>
			{#each Object.entries(f.title) as [l, text] (l)}<span class="line"><span class="muted">{l}</span> {text}</span>{/each}
		</dd>
		<dt>{m['creator.experiences_toons']()}</dt>
		<dd>{#if toons}{toons}{:else}<span class="muted">{m['creator.experiences_toons_none']()}</span>{/if}</dd>
		<dt>{m['creator.experiences_source']()}</dt>
		<dd>
			{#if f.source}
				{m['creator.experiences_source_line']({ experience: f.source.experience, build: f.source.build, schema: f.source.schema, app: f.source.app })}{#if f.source.scene} · {m['creator.experiences_source_scene']({ scene: f.source.scene })}{/if}
			{:else}<span class="muted">{m['creator.experiences_source_none']()}</span>{/if}
		</dd>
		<dt>{m['creator.experiences_assets']()}</dt>
		<dd><code>{f.assets}</code></dd>
	</dl>

	<ExperienceOutline {content} assets={f.assets} />

	<Section id="xr-versions" label={m['creator.experiences_versions']()} summary={String(versions.length)}>
		<ol class="versions" data-testid="experience-versions">
			{#each versions as v (v.id)}
				<li>
					<span>{when(v.created_at)}</span>
					{#if v.id === newest}<span class="tag">{m['creator.history_current']()}</span>{/if}
					{#if v.id === published}<span class="tag">{m['creator.experiences_published']()}</span>{/if}
					<br /><span class="muted">{v.author_email ? m['creator.history_by']({ author: v.author_email }) : m['creator.history_import']()}</span>
					{#if v.message}<br /><i>{v.message}</i>{/if}
				</li>
			{/each}
		</ol>
	</Section>
</article>

<style>
	.xr {
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
	.end {
		margin-left: auto;
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
	}
	.muted {
		color: var(--text-muted);
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
		overflow-wrap: anywhere;
	}
	.line {
		display: block;
	}
	.versions {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.versions li {
		padding: 0.4rem 0;
		border-top: var(--border-width-regular) solid var(--border-subtle);
	}
	.tag {
		margin-left: 0.4rem;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
</style>
