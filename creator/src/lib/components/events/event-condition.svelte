<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, VN_CONDITION_OPS, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { opWord, conditionWords } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/** "Only if" (CUTOVER 10.27): the event runs only when the reader's variable passes the test */
	let { doc, path, event, index }: { doc: ContentDoc; path: Path; event: any; index: number } = $props();

	function setCondition(on: boolean) {
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e) return;
			if (on) e.if = { variable: '', op: '==', value: '' };
			else delete e.if;
		}, `${path.join('.')}.if`);
	}
</script>

<details class="advanced only-if" open={!!event.if}>
	<summary>{m['creator.vn_only_if']()}{event.if?.variable ? ` · ${conditionWords(event.if)}` : ''}</summary>
	{#if event.if}
		<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
			<PropField {doc} path={[...path, 'if', 'variable']} label={m['creator.field_variable']()} required />
			<PropField {doc} path={[...path, 'if', 'op']} label={m['creator.vn_compare']()} type="select" options={VN_CONDITION_OPS.map((o) => ({ value: o, label: opWord(o) }))} required />
			{#if event.if.op !== 'set' && event.if.op !== 'unset'}<PropField {doc} path={[...path, 'if', 'value']} label={m['creator.field_value']()} required />{/if}
		</div>
		<button type="button" class="ghost" onclick={() => setCondition(false)}>{m['creator.vn_only_if_remove']()}</button>
	{:else}
		<button type="button" class="secondary" onclick={() => setCondition(true)} data-testid="add-condition-{index}">+ {m['creator.vn_only_if']()}</button>
	{/if}
</details>
