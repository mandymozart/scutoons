<script lang="ts">
	import type { ResolvedCredits, ResolvedEntry, Supporter } from '@scutoons/content';

	/**
	 * The credits as fine print (agents/DESIGN.md "Credits colophon", after the back of the vinyl cover): small, tight,
	 * centred type; section headings in capitals inside a 2px framed bar; names as running text, comma-separated, the
	 * roles in parentheses; venue | date inline; "Supported by" logos at the end. Input = `resolveCredits()` of one
	 * locale, rendered as it is. Chapter sections are left out (chapters appear only in the chapter list); the full
	 * credits page (`/<toon>/credits`, CreditsView) shows everything.
	 */
	let {
		credits,
		supporters = [],
		imageUrl = (path: string) => path
	}: {
		credits: ResolvedCredits;
		supporters?: Supporter[] | null;
		imageUrl?: (path: string) => string;
	} = $props();

	const labels = $derived(credits.labels);
	const sections = $derived(credits.sections.filter((s) => s.kind !== 'chapter'));
	const shownSupporters = $derived((supporters ?? []).filter((s) => s?.name?.trim()));
	const logos = $derived(shownSupporters.filter((s) => s.logo));

	const running = (entries: ResolvedEntry[]) =>
		entries.map((e) => (e.roles.length ? `${e.person.name} (${e.roles.join(', ')})` : e.person.name)).join(', ');
</script>

<div class="colophon" data-testid="credits-colophon">
	{#if credits.lead.length}
		<h2 class="bar">{labels.createdBy}</h2>
		<p>{running(credits.lead)}</p>
	{/if}

	{#each sections as section (section.id)}
		{#if section.runHeading}<h2 class="bar">{section.runHeading}</h2>{/if}
		{#if section.kind === 'toon'}
			<h2 class="bar">{section.heading}</h2>
			{#each section.groups as group (group.category)}
				<p data-category={group.category}><span class="label">{group.label}:</span> {running(group.entries)}</p>
			{/each}
		{:else if section.kind === 'song' || section.kind === 'performance'}
			<p data-section={section.id}>
				<span class="label">{section.heading}</span>
				{#if section.meta.length}<br />{section.meta.map((line) => line.value).join(' | ')}{/if}
				<br />{running(section.entries)}
			</p>
		{:else}
			<h2 class="bar">{section.heading}</h2>
			<p data-section={section.id}>{running(section.entries)}</p>
		{/if}
	{/each}

	{#if shownSupporters.length}
		<!-- supporters: the names as running text first, then their logos together in one framed box -->
		<h2 class="bar">{labels.supportedBy}</h2>
		<p data-testid="supporters">
			{#each shownSupporters as supporter, i (i)}{#if i > 0}{', '}{/if}{#if supporter.url}<a
						href={supporter.url}
						target="_blank"
						rel="noopener noreferrer">{supporter.name}</a
					>{:else}{supporter.name}{/if}{/each}
		</p>
		{#if logos.length}
			<ul class="logos" data-testid="supporter-logos">
				{#each logos as supporter, i (i)}
					<li><img src={imageUrl(supporter.logo ?? '')} alt={supporter.name} loading="lazy" /></li>
				{/each}
			</ul>
		{/if}
	{/if}
</div>

<style>
	/* fine print: ~11–12 px at 375 px (root 14 px), centred, a readable measure */
	.colophon {
		max-width: 36em;
		margin: 0 auto;
		padding: 0 var(--space-1, 1rem);
		font-family: var(--font-mono);
		font-size: 0.8125rem;
		line-height: 1.5;
		color: var(--ink, #000);
		text-align: center;
	}
	p {
		margin: 0 0 var(--space-05, 0.5rem);
	}
	/* section heading: capitals in a thin framed bar (2px rule, square) */
	.bar {
		margin: var(--space-15, 1.5rem) 0 var(--space-05, 0.5rem);
		padding: var(--space-025, 0.25rem) var(--space-05, 0.5rem);
		border: var(--border-width-regular, 2px) solid var(--ink, #000);
		font-size: 1em;
		line-height: 1.3;
		font-weight: var(--font-weight-bold, 700);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.bar:first-child {
		margin-top: 0;
	}
	.label {
		font-weight: var(--font-weight-bold, 700);
	}
	/* the supporters' logos in one box (2px rule, square), in ink at one visual height (~28–32 px on a phone) */
	.logos {
		list-style: none;
		margin: var(--space-1, 1rem) 0 0;
		padding: var(--space-15, 1.5rem) var(--space-1, 1rem);
		border: var(--border-width-regular, 2px) solid var(--ink, #000);
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		align-items: center;
		gap: var(--space-15, 1.5rem) var(--space-3, 3rem);
	}
	.logos img {
		display: block;
		height: clamp(28px, 2.125rem, 36px);
		width: auto;
		filter: brightness(0);
	}
	.colophon a {
		color: var(--ink, #000);
	}
</style>
