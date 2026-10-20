<script lang="ts">
	// A generation's status (#125; VARIANTS-CANVAS §3.3, owner 2026-10-20: "… bullets with icons … the icon is enough"):
	// ONE square bullet, icon only – on a card's picture (top left), beside the letter in Compare, on the clip timeline's
	// Generated clip. ✓ chosen · ? review (on the badge tokens, the glyph in ink) · ✕ rejected recedes (a muted frame, a
	// muted ✕, as on its strip). No status = nothing. `?` not an eye: in editors an eye means shown / hidden.
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import type { CardStatus } from '$lib/editor/storyboard-board-layout.ts';

	let { status, title }: { status: CardStatus | undefined; title?: string } = $props();
	/** the house chips' fills (tools.css .sc-chip.is-*) – the rejected one recedes: a muted frame */
	const CHIP = { chosen: 'sc-chip is-published', review: 'sc-chip is-warning', rejected: '' };
	const WORDS = { chosen: () => m['storyboard.status_chosen'](), review: () => m['storyboard.status_review'](), rejected: () => m['storyboard.status_rejected']() };
</script>

{#if status}
	<span class="stat {status} {CHIP[status]}" role="img" aria-label={title ?? WORDS[status]()} title={title ?? WORDS[status]()} data-status={status}>
		{#if status === 'chosen'}<Icon name="check" size="0.95rem" />{:else if status === 'review'}<Icon name="question" size="0.95rem" />{:else}<Icon name="x" size="0.95rem" /><!-- close-ok: the rejected status, not a close -->{/if}
	</span>
{/if}

<style>
	.stat {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 1.25rem;
		height: 1.25rem;
		padding: 0;
		color: var(--ink);
	}
	.rejected {
		background: var(--paper);
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--text-muted);
	}
</style>
