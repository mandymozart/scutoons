<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { GLOBAL_VARIABLES, variableRef, type Path } from '@scutoons/content';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A story variable (owner 2026-10-06, @scutoons/content variables.ts): two fields – whose variable it is (this toon,
	 * global, another toon; with `scope`: also the reader's name) and its name, a typeahead of the names that domain
	 * already has (typing a new one makes it). Stored as written: `x`, `global.x`, `<toon>.x`.
	 */
	let {
		doc,
		path,
		label = m['creator.field_variable'](),
		domainLabel = m['creator.variable_domain'](),
		scope,
		testid = ''
	}: {
		doc: ContentDoc;
		/** where the written name is stored */
		path: Path;
		label?: string;
		domainLabel?: string;
		/** askText: the scope's path – the domain select also offers the reader's name (`scope: 'reader'`) */
		scope?: Path;
		testid?: string;
	} = $props();

	const written = $derived(String(doc.get(doc.primary, path) ?? ''));
	const reader = $derived(!!scope && doc.get(doc.primary, scope) === 'reader');
	const toon = $derived(exprContext.toon);
	const ref = $derived(variableRef(written, toon));
	/** the select's value: 'toon', 'global', 'toon:<id>' or 'reader' */
	const domain = $derived(reader ? 'reader' : ref.domain === 'other' ? `toon:${ref.toon}` : ref.domain);
	const others = $derived(exprContext.toons.filter((t) => t.id !== toon));
	const own = $derived(exprContext.toons.find((t) => t.id === toon)?.name ?? toon);
	const id = $derived(`v-${path.join('-')}`);

	/** the names the chosen domain already has (as plain names) */
	const names = $derived(
		(exprContext.variables ?? [])
			.map((w) => variableRef(w, toon))
			.filter((r) => (domain === 'global' ? r.domain === 'global' : domain === 'toon' ? r.domain === 'toon' : r.domain === 'other' && `toon:${r.toon}` === domain))
			.map((r) => r.name)
	);

	function compose(d: string, name: string): string {
		if (d === 'global') return `${GLOBAL_VARIABLES}.${name}`;
		if (d.startsWith('toon:')) return `${d.slice(5)}.${name}`;
		return name;
	}

	function setDomain(d: string) {
		if (scope) {
			if (d === 'reader') {
				doc.set(scope, 'reader');
				doc.set(path, undefined);
				return;
			}
			if (reader) doc.set(scope, undefined);
		}
		doc.set(path, compose(d, ref.name));
	}

	function setName(name: string) {
		const typed = name.trim();
		// typed with its domain (`global.x`, `toptoy.x`): taken as written
		doc.set(path, typed.includes('.') ? typed : compose(domain, typed));
	}
</script>

<div class="field" data-testid={testid ? `${testid}-domain` : undefined}>
	<label for="{id}-domain">{scope ? m['creator.ask_scope']() : domainLabel}</label>
	<select id="{id}-domain" value={domain} onchange={(e) => setDomain(e.currentTarget.value)}>
		<option value="toon">{m['creator.variable_domain_toon']({ toon: own })}</option>
		<option value="global">{m['creator.variable_domain_global']()}</option>
		{#each others as t (t.id)}<option value="toon:{t.id}">{t.name}</option>{/each}
		{#if domain.startsWith('toon:') && !others.some((t) => `toon:${t.id}` === domain)}<option value={domain}>{domain.slice(5)} (?)</option>{/if}
		{#if scope}<option value="reader">{m['creator.ask_scope_reader']()}</option>{/if}
	</select>
</div>
{#if !reader}
	<div class="field" data-testid={testid || undefined}>
		<label for={id}>{label}</label>
		<input {id} type="text" list="{id}-names" value={ref.name} autocomplete="off" spellcheck="false" oninput={(e) => setName(e.currentTarget.value)} />
		<datalist id="{id}-names">{#each names as n (n)}<option value={n}></option>{/each}</datalist>
	</div>
{/if}

<style>
	/* the same look as prop-field.svelte */
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
</style>
