<script lang="ts">
	import { Section } from '@scutoons/ui/editor';
	import { contentUrl, experienceContents, kb, surfaceSize, type OutlineEntry } from '$lib/editor/experiences.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** An experience's contents, read from its configuration: scenes › surfaces › entries, the shared entities, the plugins. */
	let { content, assets }: { content: unknown; assets: string } = $props();

	const c = $derived(experienceContents(content));
</script>

{#snippet entries(list: OutlineEntry[])}
	<ul class="entries">
		{#each list as e}
			{@const pic = e.target ? contentUrl(assets, e.target.image, asset) : null}
			<li data-testid="experience-entry-{e.id}">
				{#if pic}<img class="pic" src={pic} alt="" loading="lazy" />{:else}<span class="pic"></span>{/if}
				<span class="what">
					<span>{e.title || e.id}</span>
					<span class="muted">
						{e.id}{#if e.target} · {m['creator.experiences_target']({ id: e.target.id })}{#if e.target.entity} · {m['creator.experiences_shows']({ entity: e.target.entity })}{/if}{#if !e.target.enabled} · {m['creator.experiences_target_off']()}{/if}{/if}
					</span>
				</span>
			</li>
		{/each}
	</ul>
{/snippet}

<Section id="xr-contents" label={m['creator.experiences_contents']()} summary={`${c.outline.scenes.length} · ${c.outline.entries}`}>
	{#if !c.outline.scenes.length}<p class="sc-empty">{m['creator.experiences_scenes_none']()}</p>{/if}
	<ul class="scenes" data-testid="experience-outline">
		{#each c.outline.scenes as s}
			<li>
				<span class="head">{s.label || s.id} <span class="muted">{s.id}</span></span>
				<ul class="surfaces">
					{#each s.surfaces as x}
						<li>
							<span class="head">{x.label || x.id} <span class="muted">{surfaceSize(x)}</span></span>
							{@render entries(x.entries)}
						</li>
					{/each}
					{#if s.loose.length}
						<li>
							<span class="head muted">{m['creator.experiences_loose']()}</span>
							{@render entries(s.loose)}
						</li>
					{/if}
				</ul>
			</li>
		{/each}
		{#if c.outline.orphans.length}
			<li>
				<span class="head muted">{m['creator.experiences_orphans']()}</span>
				{@render entries(c.outline.orphans)}
			</li>
		{/if}
	</ul>
</Section>

{#if c.entities.length}
	<Section id="xr-entities" label={m['creator.experiences_entities']()} summary={String(c.entities.length)}>
		<ul class="rows" data-testid="experience-entities">
			{#each c.entities as e (e.id)}
				<li>
					<span>{e.id}</span>
					<span class="muted">{m['creator.experiences_nodes']({ count: e.nodes })} · {e.types.map((t) => `${t.type} ${t.count}`).join(' · ')}</span>
				</li>
			{/each}
		</ul>
	</Section>
{/if}

{#if c.plugins.length}
	<Section id="xr-plugins" label={m['creator.experiences_plugins']()} summary={String(c.plugins.length)}>
		<ul class="rows" data-testid="experience-plugins">
			{#each c.plugins as p}
				<li>
					<span>{p.id}</span>
					<span class="muted">{m['creator.experiences_plugin_line']({ api: p.api, kb: kb(p.bytes) })}</span>
				</li>
			{/each}
		</ul>
	</Section>
{/if}

<style>
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.muted {
		color: var(--text-muted);
	}
	.head {
		display: block;
		padding: 0.4rem 0 0.2rem;
	}
	.surfaces,
	.entries {
		padding-left: 1rem;
	}
	.entries li {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.2rem 0;
	}
	.pic {
		flex: none;
		width: 2.5rem;
		height: 2.5rem;
		object-fit: contain;
		background: var(--surface-hover);
	}
	.what {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.what .muted {
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
	.rows li {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0 1rem;
		padding: 0.3rem 0;
		border-top: var(--border-width-regular) solid var(--border-subtle);
	}
</style>
