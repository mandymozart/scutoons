<script lang="ts" module>
	/** what the reachability check (@scutoons/content reach.ts) says about an item; null = fine */
	export type ReachMark = 'unreachable' | 'never' | 'loop' | 'stuck' | null;
</script>

<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A visible label on an item the reader can never reach, a condition that can never be true, a loop with no way out
	 * (owner 2026-10-06: "so the author can identify and delete them") or a question where the reader may get stuck (every
	 * choice conditional – the save refuses it) – in the chapter tree, the slice list and the VN event list; the Problems
	 * panel has the same problem.
	 */
	let { mark }: { mark: ReachMark } = $props();

	const word = $derived(
		mark === 'unreachable' ? m['creator.reach_unreachable']() : mark === 'never' ? m['creator.reach_never']() : mark === 'loop' ? m['creator.reach_loop']() : mark === 'stuck' ? m['creator.reach_stuck']() : ''
	);
	const hint = $derived(
		mark === 'unreachable'
			? m['creator.reach_unreachable_hint']()
			: mark === 'never'
				? m['creator.reach_never_hint']()
				: mark === 'loop'
					? m['creator.reach_loop_hint']()
					: mark === 'stuck'
						? m['creator.reach_stuck_hint']()
						: ''
	);
</script>

{#if mark}
	<span class="sc-chip {mark === 'stuck' ? 'is-error' : 'is-warning'} reach" title={hint} data-testid="reach-{mark}">{word}</span>
{/if}

<style>
	.reach {
		flex: none;
	}
</style>
