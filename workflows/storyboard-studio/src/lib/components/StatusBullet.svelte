<script>
	// A generation's status (agents/research/VARIANTS-CANVAS.md §3.3, owner 2026-10-20: "… bullets with icons … the icon
	// is enough"): ONE square bullet, icon only – no chip text, no side bar – on a card's picture (top left), beside the
	// letter in Compare, on the timeline's Generated clip. ✓ chosen · ? review (filled on the badge tokens, the glyph in
	// ink: published 11.56 : 1, warning 7.15 : 1 – review = the shot list's "change") · ✕ rejected recedes (a muted frame,
	// a muted ✕, as on its strip). No status = nothing. `? not an eye`: in editors an eye means shown / hidden.
	import { Icon } from '@scutoons/ui';

	/** @type {{ status: 'chosen' | 'review' | 'rejected' | null | undefined, title?: string }} */
	let { status, title } = $props();
	const ICONS = { chosen: 'check', review: 'question', rejected: 'x' };
	const WORDS = { chosen: 'Chosen', review: 'Review – to look at again', rejected: 'Rejected' };
</script>

<!-- an image with its words: a screen reader says the status (inside a card's or a clip's label it is that label's) -->
{#if status && ICONS[status]}<span class="stat {status}" role="img" aria-label={title ?? WORDS[status]} title={title ?? WORDS[status]} data-status={status}><Icon name={ICONS[status]} size="0.95rem" /></span>{/if}

<style>
	.stat {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 1.25rem;
		height: 1.25rem;
		color: var(--ink);
	}
	.chosen {
		background: var(--badge-published);
	}
	.review {
		background: var(--badge-warning);
	}
	.rejected {
		background: var(--paper);
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--text-muted);
	}
</style>
