<script lang="ts">
	import type { DeadItem, DeadPlace, DeadThings } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Dead things under the season / toon / SCUTOONS graph (owner 2026-10-06, @scutoons/content `deadThings`): one muted
	 * line with the counts, open = the lists (where each one is; a place opens its chapter). Nothing dead: nothing shown.
	 */
	let { dead, onopen }: { dead: DeadThings | null | undefined; onopen?: (at: DeadPlace) => void } = $props();

	const sections = $derived(
		dead
			? ([
					['unread', m['creator.flow_dead_unread'](), dead.unread],
					['unset', m['creator.flow_dead_unset'](), dead.unset],
					['points', m['creator.flow_dead_points'](), dead.points],
					['labels', m['creator.flow_dead_labels'](), dead.labels]
				] as [string, string, DeadItem[]][])
			: []
	);
	const total = $derived(sections.reduce((n, [, , list]) => n + list.length, 0));
	const where = (at: DeadPlace) => m['creator.flow_at']({ chapter: at.chapter, group: at.label ?? `#${at.group + 1}` });
</script>

{#if dead && total}
	<details class="dead" data-testid="flow-dead">
		<summary>
			{m['creator.flow_dead']()}:
			{m['creator.flow_dead_summary']({ unread: dead.unread.length, unset: dead.unset.length, points: dead.points.length, labels: dead.labels.length })}
		</summary>
		{#each sections as [key, title, list] (key)}
			{#if list.length}
				<p class="title">{title}</p>
				<ul>
					{#each list as item, i (`${item.name}-${i}`)}
						<li data-testid="flow-dead-{key}">
							<code>{item.name}</code>
							{#each item.at.slice(0, 3) as at, j (j)}
								<button type="button" class="link" onclick={() => onopen?.(at)} disabled={!onopen}>{where(at)}</button>
							{/each}
							{#if item.at.length > 3}<span class="more">+{item.at.length - 3}</span>{/if}
						</li>
					{/each}
				</ul>
			{/if}
		{/each}
	</details>
{/if}

<style>
	.dead {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		margin: var(--space-05) 0 0;
		max-height: 40vh;
		overflow: auto;
	}
	.title {
		margin: var(--space-05) 0 var(--space-025);
		color: var(--ink);
	}
	ul {
		margin: 0;
		padding: 0 0 0 var(--space-1);
	}
	li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-025) var(--space-05);
	}
	.link {
		background: none;
		border: 0;
		padding: 0;
		min-height: 0;
		font: inherit;
		color: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.link:disabled {
		text-decoration: none;
		cursor: default;
	}
</style>
