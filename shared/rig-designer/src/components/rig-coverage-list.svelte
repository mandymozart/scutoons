<script lang="ts">
	// The characters without a default rig (#283, RIG-EDITING.md §6: "the Sound page and the creator's character page show
	// every character without a default rig"): the core's coverage (space-web `rig_coverage`) as a list of links to each
	// one's rig; a drafted one says so. Nothing to show = nothing shown (DESIGN "Empty states").
	import type { RigCoverage } from '../assignment.ts';
	import { m } from '../messages.ts';

	let { list, current = '', href }: {
		list: RigCoverage[];
		/** the open rig's subject (marked) */
		current?: string;
		href: (c: RigCoverage) => string;
	} = $props();

	const missing = $derived(list.filter((c) => c.missing));
</script>

{#if missing.length > 0}
	<section class="missing" data-testid="rig-coverage">
		<h3>{m['designer.assign_without']({ count: missing.length })}</h3>
		<ul>
			{#each missing as c (c.character)}
				<li class:on={c.subject === current} data-testid="rig-coverage-item" data-character={c.character}>
					<a href={href(c)}>{c.character}</a>{#if c.default === 'drafted'} <span class="muted">· {m['designer.assign_draft_only']()}</span>{/if}
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.missing {
		padding: var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
	h3 {
		margin: 0 0 var(--space-05);
		font-size: 0.75rem;
		font-weight: inherit;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label, 0.08em);
		color: var(--text-muted);
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025) var(--space-1);
	}
	li.on a {
		background: var(--ink);
		color: var(--paper);
	}
	.muted {
		margin-left: var(--space-025);
		color: var(--text-muted);
	}
</style>
