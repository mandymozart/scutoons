<script lang="ts">
	import { Icon, PropRow, Segmented } from '@scutoons/ui';
	import { getAt, isDrawnBubble, tailKindOf, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { SLICE_RATIO } from '$lib/editor/defaults.ts';
	import { drawnBox, resetTail, setTailKind } from '$lib/editor/tail-edit.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * "Zeiger / Tail" (owner 2026-10-18): a drawn bubble's tail kind – none, the speech wedge or the thought circles – as
	 * icon segments, and a reset (a new tail of its kind). Its tip and base are dragged on the canvas.
	 */
	let { doc, path }: { doc: ContentDoc; path: Path } = $props();

	const dialog = $derived(doc.get(doc.primary, path));
	const kind = $derived(tailKindOf(dialog));
	const ratio = $derived.by(() => {
		const slice = doc.get(doc.primary, path.slice(0, -2));
		return slice?.width > 0 && slice?.height > 0 ? slice.height / slice.width : SLICE_RATIO;
	});
	const KINDS = ['none', 'speech', 'thought'] as const;

	function edit(change: (d: any) => void) {
		doc.edit((file) => {
			const d = getAt(file, path);
			if (d?.rect) change(d);
		}, `${path.join('.')}.tail`);
	}
</script>

{#if dialog && isDrawnBubble(dialog)}
	<PropRow label={m['creator.tail']()} testid="dialog-tail">
		<Segmented
			label={m['creator.tail']()}
			stretch
			square
			radio
			value={kind}
			onchange={(k) => edit((d) => setTailKind(d, k, drawnBox(path, d), ratio))}
			options={KINDS.map((k) => ({ value: k, icon: `tail-${k}`, title: m[`creator.tail_${k}`](), testid: `dialog-tail-${k}` }))}
			testid="dialog-tail-kind"
		/>
		<button
			type="button"
			class="sc-round"
			data-prop-fit
			title={m['creator.tail_reset']()}
			aria-label={m['creator.tail_reset']()}
			disabled={kind === 'none'}
			onclick={() => edit((d) => resetTail(d, drawnBox(path, d), ratio))}
			data-testid="dialog-tail-reset"><Icon name="rotate-ccw" /></button
		>
	</PropRow>
{/if}
