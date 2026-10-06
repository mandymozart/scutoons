<script lang="ts">
	import { onMount } from 'svelte';
	import { clickOutside, Segmented } from '@scutoons/ui';
	import { EXPR_FUNCTIONS, EXPR_NAMESPACES, evaluateExpression, evaluateText, showValue, type ExprFunction, type ExprParam } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { BUILDER_LISTS, BUILDER_OPS, codeOf, expressionView, modelOf, textView, type BuilderKind, type BuilderModel, type BuilderOp } from '$lib/editor/expr-edit.ts';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { exprTextAt, listWord } from '$lib/editor/expr-messages.ts';
	import { SAMPLE_READERS, sampleResolver, type SampleReaderId } from '$lib/editor/sample-readers.ts';

	/**
	 * The expression builder (agents/EXPRESSIONS.md §4, step 4): an in-page popover at the caret / chip that writes ink
	 * syntax without knowing it – what (show a value · if … then … else … · changes per visit · code), the value (one
	 * search over the toon's story variables, the reader state and the functions, with their descriptions), a second
	 * value with an operator, the then / else texts or the list's items (in the field's language), and a live preview
	 * for a sample reader with the resulting code. Escape closes, Enter applies (Enter on a highlighted search hit picks
	 * it). An existing chip opens parsed back when the builder can show it, else as code.
	 */
	let {
		code,
		expression = false,
		lang,
		anchor,
		onapply,
		onclose
	}: {
		/** the chip's code (`{…}`), a condition's expression, or '' for a new one */
		code: string;
		expression?: boolean;
		/** the text's language: the preview */
		lang: string;
		anchor: DOMRect;
		onapply: (code: string) => void;
		onclose: () => void;
	} = $props();

	const uid = $props.id();
	// svelte-ignore state_referenced_locally
	let model = $state<BuilderModel>(modelOf(code, expression));
	const result = $derived(codeOf(model, expression));
	const ui = getLocale() === 'de' ? 'de' : 'en';

	// ── the value search: story variables, reader state, functions ──────────────────────────────────────────────────
	interface Entry {
		/** what the value becomes (a function: computed on pick – it can wrap the value there) */
		insert: string;
		label: string;
		detail: string;
		group: string;
		fn?: ExprFunction;
	}
	const placeholderOf = (p: ExprParam) => (p.type === 'string' ? '""' : p.type === 'number' ? '0' : '""');
	const entries = $derived.by<Entry[]>(() => {
		const out: Entry[] = [];
		for (const name of exprContext.variables ?? []) out.push({ insert: name, label: name, detail: '', group: m['creator.expr_group_story']() });
		for (const ns of EXPR_NAMESPACES) {
			for (const member of ns.members) {
				const name = `${ns.name}.${member.name}`;
				const call = member.kind === 'function' ? `${name}(${(member.params ?? []).map(placeholderOf).join(', ')})` : name;
				out.push({ insert: call, label: member.kind === 'function' ? `${name}(${(member.params ?? []).map((p) => p.name).join(', ')})` : name, detail: member.description[ui], group: ns.description[ui] });
			}
		}
		for (const fn of EXPR_FUNCTIONS) {
			out.push({ insert: '', label: `${fn.name}(${fn.params.map((p) => p.name).join(', ')})`, detail: `${fn.description[ui]} · ${fn.example}`, group: m['creator.expr_group_functions'](), fn });
		}
		return out;
	});
	let searching = $state(false);
	let highlight = $state(-1);
	const hits = $derived.by(() => {
		const q = model.left.trim().toLowerCase();
		const list = q ? entries.filter((e) => e.label.toLowerCase().includes(q) || e.detail.toLowerCase().includes(q)) : entries;
		return list.filter((e) => e.insert !== model.left.trim()).slice(0, 60);
	});

	/** the value there is when it was picked or came with the chip – a function picked then wraps it: `clock(time)` */
	// svelte-ignore state_referenced_locally
	let whole = model.left.trim();
	function pick(entry: Entry) {
		if (entry.fn) {
			const [first, ...rest] = entry.fn.params;
			const current = model.left.trim();
			const wrap = current && current === whole ? current : '';
			model.left = `${entry.fn.name}(${(first ? [wrap || placeholderOf(first), ...rest.map(placeholderOf)] : []).join(', ')})`;
		} else model.left = entry.insert;
		whole = model.left;
		searching = false;
		highlight = -1;
		leftInput?.focus();
	}

	// ── kinds ───────────────────────────────────────────────────────────────────────────────────────────────────────
	const kinds = $derived<{ value: BuilderKind; label: string; title: string; testid: string }[]>(
		(expression ? (['value', 'code'] as const) : (['value', 'cond', 'list', 'code'] as const)).map((k) => ({
			value: k,
			label: { value: m['creator.expr_kind_value_short'](), cond: m['creator.expr_kind_cond_short'](), list: m['creator.expr_kind_list_short'](), code: m['creator.expr_kind_code']() }[k],
			title: { value: m['creator.expr_kind_value'](), cond: m['creator.expr_kind_cond'](), list: m['creator.expr_kind_list'](), code: m['creator.expr_kind_code']() }[k],
			testid: `expr-kind-${k}`
		}))
	);
	function setKind(kind: BuilderKind) {
		if (kind === model.kind) return;
		// to code: what was built so far; from code: read back what can be
		if (kind === 'code') model = { ...model, kind, code: result };
		else if (model.kind === 'code') {
			const back = modelOf(model.code, expression);
			model = back.kind === 'code' ? { ...modelOf('', expression), kind } : { ...back, kind };
		} else model = { ...model, kind };
	}
	const OP_WORDS: Record<BuilderOp, () => string> = {
		'==': () => m['creator.expr_op_eq'](),
		'!=': () => m['creator.expr_op_ne'](),
		'<': () => m['creator.expr_op_lt'](),
		'<=': () => m['creator.expr_op_le'](),
		'>': () => m['creator.expr_op_gt'](),
		'>=': () => m['creator.expr_op_ge'](),
		'+': () => m['creator.expr_op_add'](),
		'-': () => m['creator.expr_op_sub'](),
		'*': () => m['creator.expr_op_mul'](),
		'/': () => m['creator.expr_op_div'](),
		'%': () => m['creator.expr_op_mod']()
	};

	// ── preview ─────────────────────────────────────────────────────────────────────────────────────────────────────
	const sample = $derived<SampleReaderId>(exprContext.sample === 'written' ? 'guest' : exprContext.sample);
	const SAMPLE_WORDS: Record<SampleReaderId, () => string> = {
		guest: () => m['creator.expr_sample_guest'](),
		halfway: () => m['creator.expr_sample_halfway'](),
		complete: () => m['creator.expr_sample_complete']()
	};
	const preview = $derived.by(() => {
		if (!result.trim()) return '';
		const resolver = sampleResolver(sample, lang);
		if (expression) {
			const v = evaluateExpression(result, resolver).value;
			return typeof v === 'boolean' ? (v ? m['creator.yes']() : m['creator.no']()) : showValue(v);
		}
		return evaluateText(result, resolver).text;
	});
	const problems = $derived.by(() => {
		const known = exprContext.variables ?? undefined;
		const view = expression ? expressionView(result, known) : textView(result, { knownNames: known });
		return view.marks.map((x) => ({ level: x.level, text: exprTextAt(x) }));
	});

	// ── keys, place ─────────────────────────────────────────────────────────────────────────────────────────────────
	let box: HTMLDivElement | null = $state(null);
	let leftInput: HTMLInputElement | null = $state(null);
	let place = $state<{ left: string; top: string }>({ left: '0', top: '0' });

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			e.stopPropagation();
			onclose();
			return;
		}
		const inSearch = e.target === leftInput && searching && hits.length > 0;
		if (inSearch && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
			e.preventDefault();
			highlight = (highlight + (e.key === 'ArrowDown' ? 1 : hits.length - 1) + (highlight < 0 && e.key === 'ArrowUp' ? 1 : 0)) % hits.length;
			document.getElementById(`${uid}-hit-${highlight}`)?.scrollIntoView({ block: 'nearest' });
			return;
		}
		if (e.key === 'Enter' && !e.shiftKey && !(e.target instanceof HTMLButtonElement) && !(e.target instanceof HTMLSelectElement)) {
			e.preventDefault();
			if (inSearch && highlight >= 0) return pick(hits[highlight]);
			onapply(result);
		}
	}

	const rootPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	/** below the anchor, else above it; always inside the window (again whenever the builder's size changes) */
	function position() {
		const r = box?.getBoundingClientRect();
		if (!r) return;
		const px = rootPx();
		const gap = px * 0.25;
		const left = Math.max(gap, Math.min(anchor.left, window.innerWidth - r.width - gap));
		const below = anchor.bottom + gap;
		let top = below + r.height <= window.innerHeight - gap ? below : anchor.top - r.height - gap;
		top = Math.max(gap, Math.min(top, window.innerHeight - r.height - gap));
		place = { left: `${left / px}rem`, top: `${top / px}rem` };
	}
	onMount(() => {
		position();
		(box?.querySelector<HTMLElement>('[data-autofocus]') ?? leftInput)?.focus();
		const observer = new ResizeObserver(() => position());
		if (box) observer.observe(box);
		return () => observer.disconnect();
	});

	/** rendered into the tool's root: the popover isn't clipped by the inspector's scrolling panels */
	function portal(node: HTMLElement) {
		const target = document.querySelector('.sc-tool') ?? document.body;
		target.appendChild(node);
		return () => node.remove();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="expr-builder"
	role="dialog"
	aria-label={m['creator.expr_builder']()}
	tabindex="-1"
	bind:this={box}
	style:left={place.left}
	style:top={place.top}
	onkeydown={onKeydown}
	{@attach portal}
	{@attach clickOutside(() => onclose(), { escape: false })}
	data-testid="expr-builder"
>
	<Segmented label={m['creator.expr_kind']()} value={model.kind} onchange={(k: BuilderKind) => setKind(k)} options={kinds} />

	{#if model.kind === 'value' || model.kind === 'cond'}
		<div class="field">
			<label for="{uid}-left">{m['creator.expr_value']()}</label>
			<input
				id="{uid}-left"
				bind:this={leftInput}
				type="text"
				autocomplete="off"
				spellcheck="false"
				role="combobox"
				aria-expanded={searching && hits.length > 0}
				aria-controls="{uid}-hits"
				aria-activedescendant={highlight >= 0 ? `${uid}-hit-${highlight}` : undefined}
				placeholder={m['creator.expr_value_search']()}
				bind:value={model.left}
				onfocus={() => (searching = true)}
				oninput={() => ((searching = true), (highlight = -1))}
				data-testid="expr-left"
			/>
			{#if searching && hits.length}
				<ul class="hits" id="{uid}-hits" role="listbox" data-testid="expr-hits">
					{#each hits as entry, i (entry.group + entry.label)}
						{#if i === 0 || hits[i - 1].group !== entry.group}<li class="group" role="presentation">{entry.group}</li>{/if}
						<li id="{uid}-hit-{i}" role="option" aria-selected={i === highlight}>
							<button type="button" class="hit" class:active={i === highlight} onmousedown={(e) => e.preventDefault()} onclick={() => pick(entry)} title={entry.detail || undefined} data-testid="expr-hit">
								<span class="code">{entry.label}</span>{#if entry.detail}<span class="detail">{entry.detail}</span>{/if}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
		<div class="row">
			<div class="field">
				<label for="{uid}-op">{m['creator.expr_op']()}</label>
				<select id="{uid}-op" bind:value={model.op} onfocus={() => (searching = false)} data-testid="expr-op">
					<option value="">{m['creator.expr_op_none']()}</option>
					{#each BUILDER_OPS as op (op)}<option value={op}>{OP_WORDS[op]()}</option>{/each}
				</select>
			</div>
			{#if model.op}
				<div class="field">
					<label for="{uid}-right">{m['creator.expr_second']()}</label>
					<input id="{uid}-right" type="text" autocomplete="off" spellcheck="false" list="{uid}-names" placeholder={m['creator.expr_second_hint']()} bind:value={model.right} onfocus={() => (searching = false)} data-testid="expr-right" />
					<datalist id="{uid}-names">{#each entries.filter((e) => !e.fn) as e (e.label)}<option value={e.insert}></option>{/each}</datalist>
				</div>
			{/if}
		</div>
		{#if model.kind === 'cond'}
			<div class="row">
				<div class="field">
					<label for="{uid}-then">{m['creator.expr_then']()} · {lang}</label>
					<input id="{uid}-then" type="text" {lang} bind:value={model.then} onfocus={() => (searching = false)} data-testid="expr-then" />
				</div>
				<div class="field">
					<label for="{uid}-else">{m['creator.expr_else']()} · {lang}</label>
					<input id="{uid}-else" type="text" {lang} bind:value={model.else} onfocus={() => (searching = false)} data-testid="expr-else" />
				</div>
			</div>
		{/if}
	{:else if model.kind === 'list'}
		<div class="field">
			<label for="{uid}-list">{m['creator.expr_list_kind']()}</label>
			<select id="{uid}-list" bind:value={model.list} data-autofocus data-testid="expr-list-kind">
				{#each BUILDER_LISTS as k (k)}<option value={k}>{listWord(k)}</option>{/each}
			</select>
		</div>
		{#each model.items as _, i (i)}
			<div class="item">
				<input type="text" {lang} aria-label={m['creator.expr_item']({ n: i + 1 })} placeholder={m['creator.expr_item']({ n: i + 1 })} bind:value={model.items[i]} data-testid="expr-item-{i}" />
				<button type="button" class="sc-round ghost" title={m['creator.expr_remove_item']()} aria-label={m['creator.expr_remove_item']()} disabled={model.items.length < 2} onclick={() => model.items.splice(i, 1)}>−</button>
			</div>
		{/each}
		<button type="button" class="secondary add" onclick={() => model.items.push('')} data-testid="expr-add-item">+ {m['creator.expr_add_item']()}</button>
	{:else}
		<div class="field">
			<label for="{uid}-code">{m['creator.expr_code']()}</label>
			<input id="{uid}-code" class="mono" type="text" autocomplete="off" spellcheck="false" bind:value={model.code} data-autofocus data-testid="expr-code-input" />
		</div>
	{/if}

	<div class="preview">
		<div class="row">
			<div class="field">
				<label for="{uid}-sample">{m['creator.expr_preview_as']()}</label>
				<select id="{uid}-sample" value={sample} onchange={(e) => (exprContext.sample = e.currentTarget.value as SampleReaderId)} data-testid="expr-sample">
					{#each SAMPLE_READERS as id (id)}<option value={id}>{SAMPLE_WORDS[id]()}</option>{/each}
				</select>
			</div>
		</div>
		<p class="result" {lang} data-testid="expr-preview">{preview || m['creator.expr_preview_empty']()}</p>
		{#if model.kind !== 'code'}<code class="code-line" data-testid="expr-code">{result}</code>{/if}
		{#each problems as p, i (i)}<p class="problem {p.level}" data-testid="expr-problem">{p.text}</p>{/each}
	</div>

	<div class="actions">
		<button type="button" class="ghost" onclick={onclose}>{m['creator.cancel']()}</button>
		<button type="button" class="primary" onclick={() => onapply(result)} data-testid="expr-apply">{m['creator.expr_apply']()}</button>
	</div>
</div>

<style>
	.expr-builder {
		position: fixed;
		z-index: 60;
		box-sizing: border-box;
		width: min(28rem, calc(100vw - 1rem));
		max-height: calc(100vh - 1rem);
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		padding: var(--space-1);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		box-shadow: 0.25rem 0.25rem 0 var(--ink);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
		flex: 1;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.row {
		display: flex;
		gap: var(--space-05);
		align-items: flex-end;
	}
	.hits {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: 12rem;
		overflow: auto;
		border: var(--border-width-hairline) solid var(--border-muted);
	}
	.group {
		padding: var(--space-025) var(--space-05) 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.hit {
		all: unset;
		box-sizing: border-box;
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-05);
		width: 100%;
		padding: var(--space-025) var(--space-05);
		cursor: pointer;
	}
	.hit:hover,
	.hit.active {
		background: var(--surface-hover);
	}
	.hit:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.detail {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-body-line);
	}
	.item {
		display: flex;
		gap: var(--space-05);
		align-items: center;
	}
	.item input {
		flex: 1;
		min-width: 0;
	}
	.add {
		align-self: flex-start;
	}
	.preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding-top: var(--space-075);
		border-top: var(--border-width-hairline) solid var(--border-subtle);
	}
	.result {
		margin: 0;
		padding: var(--space-05);
		background: var(--surface-panel);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.code-line {
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		overflow-wrap: anywhere;
	}
	.problem {
		margin: 0;
		padding-left: var(--space-05);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		border-left: 0.5rem solid var(--state-warning);
	}
	.problem.error {
		border-left-color: var(--state-error);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-05);
	}
</style>
