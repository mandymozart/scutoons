<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { actionsOf, clone, getAt, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { ACTION_KINDS, actionKind, actionPath, newAction } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import LocaleText from '$lib/components/fields/locale-text.svelte';
	import EventTarget from '$lib/components/events/event-target.svelte';

	/**
	 * addChoice: the label and what the choice does, in order (CUTOVER 10.27) – set / add a variable, jump to a group,
	 * continue at a point. One action is stored as an object, more as a list.
	 */
	let {
		doc,
		path,
		event,
		index,
		pointNames,
		groupLabels
	}: { doc: ContentDoc; path: Path; event: any; index: number; pointNames: string[]; groupLabels: string[] } = $props();

	function editActions(change: (list: any[]) => void) {
		doc.edit((file) => {
			const params = getAt(file, [...path, 'params']);
			if (!params) return;
			const list = actionsOf(params.onSelect).map((a: any) => clone(a));
			change(list);
			if (!list.length) delete params.onSelect;
			else params.onSelect = list.length === 1 ? list[0] : list;
		}, `${path.join('.')}.actions`);
	}
</script>

<LocaleText {doc} path={[...path, 'params', 'label']} label={m['creator.field_text']()} multiline={false} expressions />
<div class="actions" data-testid="choice-actions-{index}">
	<span class="sc-label">{m['creator.vn_actions']()}</span>
	{#each actionsOf(event.params?.onSelect) as any[] as action, ai (ai)}
		{@const ap = actionPath(event, path, ai)}
		<div class="action">
			<select value={actionKind(action)} onchange={(e) => editActions((list) => (list[ai] = newAction(e.currentTarget.value, pointNames)))} aria-label={m['creator.vn_action']()}>
				{#each ACTION_KINDS as k (k)}<option value={k}>{m[`creator.vn_action_${k}`]()}</option>{/each}
			</select>
			<div class="sc-fields action-fields" data-cols="3" data-density="dense" data-align="end">
				{#if action?.set}
					<PropField {doc} path={[...ap, 'set', 'variable']} label={m['creator.field_variable']()} required />
					<PropField {doc} path={[...ap, 'set', 'value']} label={m['creator.field_value']()} required />
				{:else if action?.add}
					<PropField {doc} path={[...ap, 'add', 'variable']} label={m['creator.field_variable']()} required />
					<PropField {doc} path={[...ap, 'add', 'value']} label={m['creator.field_value']()} type="number" step={1} required />
				{:else if action?.jumpTo}
					<EventTarget {doc} path={[...ap, 'jumpTo', 'target']} target={action.jumpTo.target} {groupLabels} />
				{:else if action?.divert}
					<PropField {doc} path={[...ap, 'divert', 'to']} label={m['creator.vn_point']()} type="select" options={pointNames} required />
				{/if}
			</div>
			<button type="button" class="ghost" onclick={() => editActions((list) => list.splice(ai, 1))} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
		</div>
	{/each}
	<button type="button" class="secondary" onclick={() => editActions((list) => list.push(newAction(list.length ? 'jumpTo' : 'set', pointNames)))} data-testid="add-action-{index}">+ {m['creator.vn_action']()}</button>
</div>

<style>
	.actions {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.action {
		display: grid;
		grid-template-columns: 9rem minmax(0, 1fr) auto;
		gap: 0.4rem;
		align-items: end;
		padding: 0.3rem 0;
		border-top: 1px dashed var(--border-muted);
	}
	.action-fields {
		min-width: 0;
	}
</style>
