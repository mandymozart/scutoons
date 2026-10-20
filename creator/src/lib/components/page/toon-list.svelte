<script lang="ts">
	import { Badge, ToonAvatar } from '@scutoons/ui';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { statusWord } from '$lib/editor/status-word.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * Every toon as one compact row – avatar · name · state (owner 2026-10-12: "Platz sparen", no id, the state as a
	 * filled label); the Toons page and the dashboard's Toons. Rows split by 1px ink like the toon page's chapter list.
	 */
	let { toons, columns = false }: { toons: ToonIndex[]; columns?: boolean } = $props();
	const name = (toon: ToonIndex) => toonName(toon, getLocale());
</script>

<ul class="toons" class:columns aria-label={m['creator.toons']()}>
	{#each toons as toon (toon.id)}
		<li>
			<a href="/{toon.id}" data-testid="toon-{toon.id}">
				<ToonAvatar src={toon.avatar ? asset(`${toon.id}/${toon.avatar}`) : null} name={name(toon)} size="2rem" />
				<span class="name">{name(toon)}</span>
				<Badge tone={toon.status === 'published' ? 'published' : toon.status === 'coming-soon' ? 'soon' : 'draft'}>{statusWord(toon.status)}</Badge>
			</a>
		</li>
	{/each}
</ul>

<style>
	.toons {
		list-style: none;
		padding: 0;
		margin: 0;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	/* the Toons page: two or more columns when there is room, one on a phone */
	.columns {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(18rem, 100%), 1fr));
		column-gap: 1.5rem;
	}
	li {
		border-bottom: var(--border-width-hairline) solid var(--ink);
	}
	a {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-height: var(--control-height);
		padding: var(--space-025) 0;
		color: var(--ink);
		text-decoration: none;
	}
	a:hover .name {
		text-decoration: underline;
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}
</style>
