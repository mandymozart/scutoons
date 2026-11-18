<script lang="ts">
	// The docs' index: every use case of a creator, grouped by the phase of making a toon; a written one links its tutorial.
	import { OFTEN, WHERE, WHO, written } from '$lib/docs/catalog.ts';
	import { GROUPS } from '$lib/docs/tutorials.ts';
	import Chips from '$lib/docs/chips.svelte';
</script>

<svelte:head>
	<title>Creator docs · SCUTOONS</title>
	<meta name="description" content="How to make a toon with SCUTOONS: short step-by-step guides for the creator and the deck." />
</svelte:head>

<header class="head">
	<p class="eyebrow">SCUTOONS · Creator docs</p>
	<h1>How to make a toon</h1>
	<p>Short guides, step by step, for the creator (creator.scutoons.com) and the deck. Each one is a thing creators do; the phases follow the making of a toon.</p>
	<nav aria-label="Phases">
		<ol class="toc">
			{#each GROUPS as group, i (group.key)}
				<li><a href="#{group.key}">{i + 1} · {group.title}</a></li>
			{/each}
		</ol>
	</nav>
</header>

{#each GROUPS as group, i (group.key)}
	<section id={group.key}>
		<h2>{i + 1} · {group.title}</h2>
		<p class="intro">{group.intro}</p>
		<ul class="cases">
			{#each group.cases as item (item.slug)}
				<li>
					{#if written(item)}
						<a class="card" href="/docs/{item.slug}/" data-testid="docs-case">
							<span class="title">{item.title}</span>
							<span class="summary">{item.summary}</span>
							<Chips {item} />
						</a>
					{:else}
						<div class="card soon" data-testid="docs-case-soon">
							<span class="title">{item.title}</span>
							<span class="summary">{item.summary}</span>
							<Chips {item} soon />
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	</section>
{/each}

<p class="legend">
	Who: {Object.values(WHO).join(' · ')}. Where: {Object.values(WHERE).join(' · ')}. How often: ●●● {OFTEN[3]} · ●● {OFTEN[2]} · ● {OFTEN[1]}.
</p>

<style>
	.eyebrow {
		margin: 0;
		color: var(--d-muted);
	}
	h1 {
		margin: 0.5rem 0 1rem;
		font-size: 2rem;
		line-height: 1.1;
	}
	.head > p:not(.eyebrow),
	.intro {
		max-width: 44rem;
		color: var(--d-muted);
	}
	.toc {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 1rem;
		margin: 1.5rem 0 0;
		padding: 0;
		list-style: none;
	}
	.toc a {
		text-decoration: none;
		border-bottom: 1px solid var(--d-line);
	}
	section {
		margin-top: 3rem;
		scroll-margin-top: 1rem;
	}
	h2 {
		margin: 0;
		font-size: 1.25rem;
	}
	.cases {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
		gap: 0.75rem;
		margin: 1rem 0 0;
		padding: 0;
		list-style: none;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		height: 100%;
		box-sizing: border-box;
		padding: 1rem;
		border: 2px solid var(--d-line);
		background: var(--d-card);
		text-decoration: none;
	}
	a.card:hover {
		border-color: var(--d-ink);
		background: var(--d-hover);
	}
	.card.soon {
		border-style: dashed;
		color: var(--d-muted);
	}
	.summary {
		flex: 1;
		color: var(--d-muted);
		font-size: 0.875rem;
		line-height: 1.4;
	}
	.legend {
		margin-top: 3rem;
		color: var(--d-muted);
		font-size: 0.875rem;
	}
</style>
