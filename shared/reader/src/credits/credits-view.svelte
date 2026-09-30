<script lang="ts">
	import type { ResolvedCredits, ResolvedEntry, ResolvedPerson } from '@scutoons/content';

	/**
	 * The credits page of a toon: "Created by" first (prominent), the toon credits grouped by role category, then the
	 * season / chapter / song / performance sections and special thanks. Input = `resolveCredits()` of one locale, so
	 * the viewer and the creator's preview render the same thing. `imageUrl` turns a CDN path into a URL.
	 */
	let {
		credits,
		imageUrl = (path: string) => path,
		showTitle = true
	}: {
		credits: ResolvedCredits;
		imageUrl?: (path: string) => string;
		showTitle?: boolean;
	} = $props();

	const labels = $derived(credits.labels);
</script>

{#snippet links(person: ResolvedPerson)}
	{#if person.links.length}
		<span class="links">
			{#each person.links as link (link.url)}
				<a href={link.url} target="_blank" rel="noopener noreferrer">{link.label || link.url}</a>
			{/each}
		</span>
	{/if}
{/snippet}

{#snippet table(entries: ResolvedEntry[])}
	<table>
		<tbody>
			{#each entries as entry, i (entry.person.id + ':' + i)}
				<tr data-person={entry.person.id}>
					<td>
						<span class="person">
							{#if entry.person.image}<img class="thumb" src={imageUrl(entry.person.image)} alt="" loading="lazy" />{/if}
							<span class="name">{entry.person.name}</span>
						</span>
						{@render links(entry.person)}
					</td>
					<td class="roles">{entry.roles.join(', ')}</td>
				</tr>
			{/each}
		</tbody>
	</table>
{/snippet}

<div class="credits">
	{#if showTitle}
		<header class="credits-header">
			<h1>{labels.title}</h1>
		</header>
	{/if}

	{#if credits.lead.length}
		<section class="lead" data-section="lead">
			<h2>{labels.createdBy}</h2>
			<ul>
				{#each credits.lead as entry (entry.person.id)}
					<li data-person={entry.person.id}>
						{#if entry.person.image}<img src={imageUrl(entry.person.image)} alt="" loading="lazy" />{/if}
						<div>
							<p class="lead-name">{entry.person.name}</p>
							{#if entry.roles.length}<p class="lead-roles">{entry.roles.join(', ')}</p>{/if}
							{#if entry.person.bio}<p class="bio">{entry.person.bio}</p>{/if}
							{@render links(entry.person)}
						</div>
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	{#each credits.sections as section (section.id)}
		{#if section.runHeading}<h2 class="run">{section.runHeading}</h2>{/if}
		<section class="section kind-{section.kind}" data-section={section.id}>
			{#if section.kind === 'toon'}
				<h2>{section.heading}</h2>
				{#each section.groups as group (group.category)}
					<div class="group" data-category={group.category}>
						<h3>{group.label}</h3>
						{@render table(group.entries)}
					</div>
				{/each}
			{:else if section.kind === 'thanks'}
				<h2>{section.heading}</h2>
				<p class="thanks">
					{#each section.entries as entry, i (entry.person.id + ':' + i)}<span data-person={entry.person.id}>{entry.person.name}</span
						>{#if entry.roles.length}{' '}({entry.roles.join(', ')}){/if}{i < section.entries.length - 1 ? ', ' : ''}{/each}
				</p>
			{:else}
				{#if section.kind === 'song' || section.kind === 'performance'}
					<h3>{section.heading}</h3>
				{:else}
					<h2>{section.heading}</h2>
				{/if}
				{#each section.meta as line (line.label)}
					<p class="meta"><strong>{line.label}:</strong> {line.value}</p>
				{/each}
				{@render table(section.entries)}
			{/if}
		</section>
	{/each}
</div>

<style>
	.credits {
		font-family: var(--font-mono);
		color: var(--ink, #000);
	}
	.credits-header {
		padding: 0 var(--space-1, 1rem);
	}
	section,
	.run {
		padding: var(--space-1, 1rem);
	}
	.run {
		padding-bottom: 0;
		margin: 0;
	}
	.section.kind-song,
	.section.kind-performance {
		padding-top: 0;
	}
	h2 {
		margin: 0 0 var(--space-075, 0.75rem);
	}
	h3 {
		margin: var(--space-075, 0.75rem) 0 var(--space-05, 0.5rem);
	}
	.lead ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-1, 1rem);
	}
	.lead li {
		display: flex;
		gap: var(--space-1, 1rem);
		align-items: flex-start;
		border: var(--border-width-regular, 2px) solid var(--ink, #000);
		padding: var(--space-1, 1rem);
	}
	.lead img {
		width: 5rem;
		height: 5rem;
		object-fit: cover;
		border: var(--border-width-regular, 2px) solid var(--ink, #000);
		flex: none;
	}
	.lead p {
		margin: 0;
	}
	.lead-name {
		font-size: 1.5rem;
		font-weight: var(--font-weight-bold, 700);
		line-height: 1.2;
	}
	.lead-roles,
	.bio {
		margin-top: var(--space-025, 0.25rem) !important;
	}
	.bio {
		font-size: var(--type-caption-size, 0.75rem);
		line-height: var(--type-caption-line, 1.3);
		color: var(--text-muted, #6b6b6b);
	}
	.meta {
		margin: 0 0 var(--space-025, 0.25rem);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin-bottom: var(--space-05, 0.5rem);
	}
	td {
		padding: 0.5rem;
		border: 1px solid var(--border-muted, #ccc);
		vertical-align: top;
	}
	td:first-child {
		background: var(--surface-hover, #f0f0f0);
		width: 33%;
		min-width: 33%;
	}
	.person {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05, 0.5rem);
	}
	.thumb {
		width: 1.5rem;
		height: 1.5rem;
		object-fit: cover;
		border-radius: var(--radius-sm, 0.5rem);
	}
	.links {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-05, 0.5rem);
		font-size: var(--type-caption-size, 0.75rem);
		line-height: var(--type-caption-line, 1.3);
	}
	.links a {
		color: var(--ink, #000);
	}
	.thanks {
		margin: 0;
	}
</style>
