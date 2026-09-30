<script lang="ts">
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The ActionBar's first item on every creator page: one level up (the parent crumb; Alt+ArrowUp does the same). */
	const nav = getCreatorNav();
</script>

{#if nav.parent}
	<a class="up-link" href={nav.parent.href} title="{m['creator.up']()} (Alt+ArrowUp)" data-testid="up-link">{nav.parent.label}</a>
{/if}

<style>
	.up-link {
		color: var(--ink);
		white-space: nowrap;
		max-width: 14rem;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
