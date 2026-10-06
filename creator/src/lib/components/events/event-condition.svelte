<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, isExprCondition, VN_CONDITION_OPS, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { confirmDialog } from '@scutoons/ui';
	import { opWord, conditionAsExpression, conditionWords } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';
	import ExprText from '$lib/components/fields/expr-text.svelte';
	import { fieldKey } from '$lib/editor/expr-context.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * "Only if" (CUTOVER 10.27): the event runs only when the test passes – the simple form (a variable compared with a
	 * value, the default) or an expression (agents/EXPRESSIONS.md §3: ink logic on the expression engine, answered for
	 * the reader – `count("e…") < 3 and not has("cucumber")`): the expression-aware field in expression mode (wavy
	 * underlines with the problem as tooltip, Enter = the builder; fields/expr-text.svelte).
	 */
	let { doc, path, event, index }: { doc: ContentDoc; path: Path; event: any; index: number } = $props();

	const expression = $derived(isExprCondition(event.if));
	/** the builder's preview language: the header's text language */
	const previewLang = $derived(textLanguage.visible(doc.textLanguages, doc.textSource)[0] ?? doc.textSource);

	const setExpr = (value: string) => doc.set([...path, 'if', 'expr'], value);

	function setCondition(on: boolean) {
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e) return;
			if (on) e.if = { variable: '', op: '==', value: '' };
			else delete e.if;
		}, `${path.join('.')}.if`);
	}

	/** Simple ⇄ Expression: the simple test becomes its expression; an expression back to an empty simple test (asked) */
	async function setKind(kind: string) {
		if ((kind === 'expr') === expression) return;
		if (kind === 'simple' && event.if.expr.trim()) {
			const ok = await confirmDialog({
				title: m['creator.vn_condition_simple'](),
				message: `${m['creator.vn_only_if_remove']()}: ${event.if.expr}`,
				confirmLabel: m['creator.vn_condition_simple']()
			});
			if (!ok) return;
		}
		doc.edit((file) => {
			const e = getAt(file, path);
			if (!e) return;
			e.if = kind === 'expr' ? { expr: conditionAsExpression(e.if) } : { variable: '', op: '==', value: '' };
		}, `${path.join('.')}.if`);
	}
</script>

<details class="advanced only-if" open={!!event.if}>
	<summary>{m['creator.vn_only_if']()}{conditionWords(event.if) ? ` · ${conditionWords(event.if)}` : ''}</summary>
	{#if event.if}
		<div class="sc-fields" data-cols="3" data-density="dense" data-align="end">
			<div class="field">
				<label for="if-kind-{path.join('-')}">{m['creator.vn_condition_kind']()}</label>
				<select id="if-kind-{path.join('-')}" value={expression ? 'expr' : 'simple'} onchange={(e) => setKind(e.currentTarget.value)} data-testid="condition-kind-{index}">
					<option value="simple">{m['creator.vn_condition_simple']()}</option>
					<option value="expr">{m['creator.vn_condition_expression']()}</option>
				</select>
			</div>
			{#if !expression}
				<PropField {doc} path={[...path, 'if', 'variable']} label={m['creator.field_variable']()} required />
				<PropField {doc} path={[...path, 'if', 'op']} label={m['creator.vn_compare']()} type="select" options={VN_CONDITION_OPS.map((o) => ({ value: o, label: opWord(o) }))} required />
				{#if event.if.op !== 'set' && event.if.op !== 'unset'}<PropField {doc} path={[...path, 'if', 'value']} label={m['creator.field_value']()} required />{/if}
			{/if}
		</div>
		{#if expression}
			<div class="expr">
				<label for="if-expr-{path.join('-')}">{m['creator.vn_condition_expression']()}</label>
				<ExprText
					id="if-expr-{path.join('-')}"
					value={event.if.expr ?? ''}
					onchange={setExpr}
					multiline={false}
					expression
					lang={previewLang}
					placeholder={m['creator.vn_condition_expr_hint']()}
					focusKey={fieldKey(doc.entity, [...path, 'if', 'expr'], null)}
					testid="condition-expr-{index}"
				/>
			</div>
		{/if}
		<button type="button" class="ghost" onclick={() => setCondition(false)}>{m['creator.vn_only_if_remove']()}</button>
	{:else}
		<button type="button" class="secondary" onclick={() => setCondition(true)} data-testid="add-condition-{index}">+ {m['creator.vn_only_if']()}</button>
	{/if}
</details>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	/* laid out like a PropField */
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.expr {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
	.expr label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
</style>
