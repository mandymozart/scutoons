<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { analyseExpression, getAt, isExprCondition, VN_CONDITION_OPS, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { confirmDialog } from '@scutoons/ui';
	import { opWord, conditionAsExpression, conditionWords } from '$lib/editor/vn-forms.ts';
	import PropField from '$lib/components/fields/prop-field.svelte';

	/**
	 * "Only if" (CUTOVER 10.27): the event runs only when the test passes – the simple form (a variable compared with a
	 * value, the default) or an expression (agents/EXPRESSIONS.md §3: ink logic on the expression engine, answered for
	 * the reader – `count("e…") < 3 and not has("cucumber")`). A plain text field for now; step 4 brings the builder.
	 */
	let { doc, path, event, index }: { doc: ContentDoc; path: Path; event: any; index: number } = $props();

	const expression = $derived(isExprCondition(event.if));
	/** the expression's problems while typing (syntax errors; unknown names are only doubts) */
	const problems = $derived(expression && event.if.expr.trim() ? analyseExpression(event.if.expr).errors : []);

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
				<PropField {doc} path={[...path, 'if', 'expr']} label={m['creator.vn_condition_expression']()} placeholder={m['creator.vn_condition_expr_hint']()} required testid="condition-expr-{index}" />
				{#each problems as p, pi (pi)}
					<p class={p.severity === 'error' ? 'error' : 'warn'} data-testid="condition-problem-{index}">{p.message} – {p.start + 1}{p.end > p.start + 1 ? `–${p.end}` : ''}</p>
				{/each}
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
	.error,
	.warn {
		margin: 0;
		padding-left: var(--space-05);
		font-size: var(--type-caption-size);
		border-left: 0.5rem solid var(--state-warning);
	}
	.error {
		border-left-color: var(--state-error, var(--state-warning));
	}
</style>
