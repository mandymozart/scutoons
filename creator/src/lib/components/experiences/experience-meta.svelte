<script lang="ts">
	import { crossAppClick } from '@scutoons/pwa';
	import { VIEWER_URL } from '$lib/config.ts';
	import { kb, readerHref, titleIn, type ExperienceFacts, type PluginRow } from '$lib/editor/experiences.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/** The Experience tab: the open experience's meta (what it is, for which toons, where it came from) and the reader's link. */
	let { facts, typeName, toons, problems, plugins }: { facts: ExperienceFacts; typeName: string; toons: string; problems: string[]; plugins: PluginRow[] } = $props();

	const locale = getLocale();
</script>

<div class="meta" data-testid="experience-meta">
	<p class="head">
		<span class="title">{titleIn(facts.title, locale, facts.id)}</span>
		<code class="muted">experience:{facts.id}</code>
	</p>
	{#if problems.length}
		<div class="problems" role="alert" data-testid="experience-problems">
			<p>{m['creator.experiences_problems']()}</p>
			<ul>{#each problems as p, i (i)}<li>{p}</li>{/each}</ul>
		</div>
	{/if}
	<dl class="facts" data-testid="experience-facts">
		<dt>{m['creator.experiences_type']()}</dt>
		<dd>{typeName}</dd>
		<dt>{m['creator.experiences_title']()}</dt>
		<dd>{#each Object.entries(facts.title) as [l, text] (l)}<span class="line"><span class="muted">{l}</span> {text}</span>{/each}</dd>
		<dt>{m['creator.experiences_toons']()}</dt>
		<dd>{#if toons}{toons}{:else}<span class="muted">{m['creator.experiences_toons_none']()}</span>{/if}</dd>
		<dt>{m['creator.experiences_source']()}</dt>
		<dd>
			{#if facts.source}
				{m['creator.experiences_source_line']({ experience: facts.source.experience, build: facts.source.build, schema: facts.source.schema, app: facts.source.app })}{#if facts.source.scene} · {m['creator.experiences_source_scene']({ scene: facts.source.scene })}{/if}
			{:else}<span class="muted">{m['creator.experiences_source_none']()}</span>{/if}
		</dd>
		<dt>{m['creator.experiences_assets']()}</dt>
		<dd><code>{facts.assets}</code></dd>
		<dt>{m['creator.experiences_plugins']()}</dt>
		<dd>{#if plugins.length}{#each plugins as p (p.id)}<span class="line">{p.id} <span class="muted">{m['creator.experiences_plugin_line']({ api: p.api, kb: kb(p.bytes) })}</span></span>{/each}{:else}<span class="muted">–</span>{/if}</dd>
	</dl>
	<a class="reader" href={readerHref(VIEWER_URL, facts.id)} target="_blank" rel="noopener" onclick={crossAppClick} data-testid="experience-reader">{m['creator.experiences_open_reader']()} ↗</a>
</div>

<style>
	.meta {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		margin: 0;
	}
	.title {
		font-size: var(--editor-title);
	}
	.muted {
		color: var(--tool-muted);
	}
	/* an error: the stop outline, no ground (RULES H10, the kit's field error) */
	.problems {
		padding: 0.5rem 0.75rem;
		border: var(--editor-edge) solid var(--kit-field-error-border);
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
		grid-template-columns: max-content 1fr;
		gap: 0.4rem 1rem;
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
</style>
