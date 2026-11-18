<script lang="ts">
	// One tutorial: what it is, the facts (who, where, how often), the steps, a note, the next tutorial.
	import { OFTEN, WHERE, WHO } from '$lib/docs/catalog.ts';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();
	const item = $derived(data.item);
</script>

<svelte:head>
	<title>{item.title} · Creator docs · SCUTOONS</title>
	<meta name="description" content={item.summary} />
</svelte:head>

<article>
	<a class="crumb" href="/docs/#{data.group.key}">‹ Docs</a>
	<p class="eyebrow">Docs · {data.group.title}</p>
	<h1>{item.title}</h1>
	<p class="summary">{item.summary}.</p>

	<dl class="facts">
		<dt>Who</dt>
		<dd>{item.who.map((w) => WHO[w]).join(', ')}</dd>
		<dt>Where</dt>
		<dd>{item.where.map((w) => WHERE[w]).join(', ')}</dd>
		<dt>How often</dt>
		<dd>{OFTEN[item.often]}</dd>
		{#each item.facts ?? [] as fact (fact.label)}
			<dt>{fact.label}</dt>
			<dd>{fact.text}</dd>
		{/each}
	</dl>

	{#if item.partly}
		<p class="note" data-tone="part">Built in part: these steps show what works today; more is coming.</p>
	{/if}

	<h2>Step by step</h2>
	<ol class="steps">
		{#each item.steps ?? [] as step, i (i)}
			<li id="step-{i + 1}" tabindex="-1"><p>{step.text}</p></li>
		{/each}
	</ol>

	{#if item.note}<p class="note">{item.note}</p>{/if}

	{#if data.next}
		<a class="next" href="/docs/{data.next.slug}/" data-testid="docs-next">
			<span class="label">Next guide</span>
			<span class="title">{data.next.title}</span>
			<span class="sum">{data.next.summary}</span>
		</a>
	{:else}
		<a class="next" href="/docs/" data-testid="docs-next"><span class="label">Done</span><span class="title">All guides</span></a>
	{/if}
</article>

<style>
	article {
		max-width: 44rem;
	}
	.crumb {
		text-decoration: none;
		color: var(--d-muted);
	}
	.eyebrow {
		margin: 1.5rem 0 0;
		color: var(--d-muted);
	}
	h1 {
		margin: 0.5rem 0;
		font-size: 2rem;
		line-height: 1.1;
	}
	.summary {
		color: var(--d-muted);
	}
	.facts {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 0.5rem 1.5rem;
		margin: 2rem 0;
		padding: 1rem 0;
		border-top: 2px solid var(--d-line);
		border-bottom: 2px solid var(--d-line);
	}
	.facts dt {
		color: var(--d-muted);
	}
	.facts dd {
		margin: 0;
	}
	h2 {
		margin: 2rem 0 1rem;
		font-size: 1.25rem;
	}
	.steps {
		margin: 0;
		padding: 0;
		list-style: none;
		counter-reset: step;
	}
	.steps li {
		counter-increment: step;
		display: grid;
		grid-template-columns: 2.5rem 1fr;
		gap: 0.75rem;
		padding: 1rem 0;
		border-bottom: 1px solid var(--d-line);
		scroll-margin-top: 1rem;
	}
	.steps li::before {
		content: counter(step);
		display: grid;
		place-items: center;
		width: 2rem;
		height: 2rem;
		border: 2px solid var(--d-accent);
		border-radius: 50%;
	}
	.steps li:target {
		background: var(--d-hover);
	}
	.steps p {
		margin: 0.25rem 0 0;
		line-height: 1.5;
	}
	.note {
		margin: 1.5rem 0 0;
		padding: 0.75rem 1rem;
		border-left: 0.25rem solid var(--d-accent);
		background: var(--d-card);
		line-height: 1.5;
	}
	.note[data-tone='part'] {
		border-left-style: dashed;
		color: var(--d-muted);
	}
	.next {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-top: 3rem;
		padding: 1rem;
		border: 2px solid var(--d-line);
		text-decoration: none;
	}
	.next:hover {
		border-color: var(--d-ink);
		background: var(--d-hover);
	}
	.next .label,
	.next .sum {
		color: var(--d-muted);
		font-size: 0.875rem;
	}
</style>
