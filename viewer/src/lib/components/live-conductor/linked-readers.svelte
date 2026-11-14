<script lang="ts">
	// The linked readers now (#529, agents/research/SCENE-EVENTS.md §6.3): each by the name it said and its client, the one
	// that drives the session's story, and the last story event heard (type, the characters it names) – all the core's
	// (relay-web `readers()`, deck-space linked.rs `Readers`), drawn as they come.
	import { m } from '$lib/paraglide/messages.js';
	import type { LinkedReaders } from '$lib/conductor-reader.ts';

	let { readers }: { readers: LinkedReaders | null } = $props();

	const linked = $derived(Object.entries(readers?.linked ?? {}));
	const last = $derived(readers?.last ?? null);
</script>

<section class="readers" aria-labelledby="conductor-readers-title" data-testid="conductor-readers">
	<h2 id="conductor-readers-title">{m['conductor_reader.readers']()}</h2>
	{#if linked.length}
		<ul>
			{#each linked as [client, name] (client)}
				<li data-testid="conductor-reader-linked">
					{#if name}<span>{name}</span>{/if}
					<span class="muted">{client}</span>
					{#if client === readers?.driver}<span class="drives">{m['conductor_reader.drives']()}</span>{/if}
				</li>
			{/each}
		</ul>
	{:else}
		<p class="muted">{m['conductor_reader.none']()}</p>
	{/if}
	{#if last}
		<p data-testid="conductor-reader-last">
			{m['conductor_reader.last']({ type: last.type, from: readers?.linked[last.from] ?? last.from })}
			{#if last.characters.length}<span class="muted">{last.characters.join(', ')}</span>{/if}
		</p>
	{/if}
</section>

<style>
	.readers {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	h2 {
		margin: 0;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	li {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: baseline;
	}
	p {
		margin: 0;
	}
	.muted {
		color: var(--text-muted);
	}
	.drives {
		padding: 0 var(--space-05);
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
	}
</style>
