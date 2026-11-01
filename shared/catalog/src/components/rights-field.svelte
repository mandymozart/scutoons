<script lang="ts">
	import type { Data } from '../kinds.ts';
	import { m } from '../messages.ts';
	import { HOLDS, RIGHTS } from '../rigs/validate.ts';
	import { rightsOf, withRight } from '../rigs/edit.ts';

	/**
	 * A definition's rights (research/INSTRUMENTS.md §4, mod.rs Rights): per way of holding it (lent · owned · core) the
	 * rights a deck has (play · tweak · derive · copy). The defaults are shown as checked; the field is stored only when
	 * something differs from them.
	 */
	let { data, hint, onchange, testid = 'rights' }: { data: Data; hint?: string; onchange: (next: Data) => void; testid?: string } = $props();

	const rights = $derived(rightsOf(data));
	const HOLD_LABELS = { lent: () => m['catalog.rights_lent'](), owned: () => m['catalog.rights_owned'](), core: () => m['catalog.rights_core']() } as const;
	const RIGHT_LABELS = { play: () => m['catalog.right_play'](), tweak: () => m['catalog.right_tweak'](), derive: () => m['catalog.right_derive'](), copy: () => m['catalog.right_copy']() } as const;
</script>

<fieldset class="rights" data-testid={testid}>
	<legend>{m['catalog.rights']()}</legend>
	<div class="grid" style:--cols={RIGHTS.length}>
		<span></span>
		{#each RIGHTS as right (right)}<span class="sc-caption head">{RIGHT_LABELS[right]()}</span>{/each}
		{#each HOLDS as hold (hold)}
			<span class="sc-caption">{HOLD_LABELS[hold]()}</span>
			{#each RIGHTS as right (right)}
				<label class="cell" title="{HOLD_LABELS[hold]()} · {RIGHT_LABELS[right]()}">
					<input type="checkbox" checked={rights[hold].includes(right)} onchange={(e) => onchange(withRight(data, hold, right, e.currentTarget.checked))} data-testid="{testid}-{hold}-{right}" />
				</label>
			{/each}
		{/each}
	</div>
	{#if hint}<p class="sc-caption muted">{hint}</p>{/if}
</fieldset>

<style>
	.rights {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin: 0 0 var(--space-15);
	}
	.grid {
		display: grid;
		grid-template-columns: 6rem repeat(var(--cols), minmax(3.5rem, auto));
		align-items: center;
		gap: 0 var(--space-05);
		max-width: 28rem;
	}
	.head {
		text-align: center;
	}
	.cell {
		display: flex;
		justify-content: center;
		min-height: var(--control-height);
		align-items: center;
	}
	.muted {
		color: var(--text-muted);
		margin: 0;
	}
</style>
