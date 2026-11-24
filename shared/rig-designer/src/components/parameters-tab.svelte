<script lang="ts">
	// The Parameters tab, read-only (RIG-EDITING.md §2.1): the compiled module's descriptor – its name, ABI and version, the
	// parameters with range and default, the ports. It reads what the last compile returned, or what the document stores;
	// editing a parameter's range comes with the SDK's attributes, not here. A muted line says when the files changed since.
	import type { RigDesigner } from '../designer.svelte.ts';
	import { headline, num } from '../descriptor.ts';
	import { m } from '../messages.ts';

	let { designer }: { designer: RigDesigner } = $props();
	const d = $derived(designer.descriptor);
</script>

<div class="params" data-testid="designer-parameters">
	{#if !d}
		<p class="muted">{m['designer.params_none']()}</p>
	{:else}
		<p class="head" data-testid="designer-descriptor">{headline(d)}</p>
		{#if designer.stale}<p class="muted" data-testid="designer-stale">{m['designer.params_stale']()}</p>{/if}

		<h3>{m['designer.summary_params']()}</h3>
		{#if d.params.length === 0}
			<p class="muted">{m['designer.params_empty']()}</p>
		{:else}
			<table class="values">
				<thead>
					<tr><th>{m['designer.param_name']()}</th><th>{m['designer.param_min']()}</th><th>{m['designer.param_max']()}</th><th>{m['designer.param_default']()}</th></tr>
				</thead>
				<tbody>
					{#each d.params as p (p.name)}
						<tr data-testid="designer-param"><td>{p.name}</td><td>{num(p.min)}</td><td>{num(p.max)}</td><td>{num(p.default)}</td></tr>
					{/each}
				</tbody>
			</table>
		{/if}

		<h3>{m['designer.summary_ports']()}</h3>
		<table>
			<thead>
				<tr><th>{m['designer.port_name']()}</th><th>{m['designer.port_dir']()}</th><th>{m['designer.port_kind']()}</th></tr>
			</thead>
			<tbody>
				{#each d.ports as p (p.dir + p.name)}
					<tr data-testid="designer-port"><td>{p.name}</td><td>{p.dir === 'in' ? m['designer.dir_in']() : m['designer.dir_out']()}</td><td>{p.type}{p.channels !== undefined ? ` ×${p.channels}` : ''}</td></tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<style>
	.params {
		height: 100%;
		overflow: auto;
		padding: var(--space-1);
		color: var(--tool-text);
	}
	.head {
		margin: 0 0 var(--space-05);
	}
	.muted {
		margin: 0 0 var(--space-05);
		color: var(--tool-muted);
	}
	/* a section's head: the label type, muted (deck-kit's) */
	h3 {
		margin: var(--space-1) 0 var(--space-025);
		font-size: var(--type-label-size);
		font-weight: inherit;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--tool-muted);
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		height: var(--editor-tree-row);
		padding: 0 var(--space-05) 0 0;
		text-align: left;
		font-weight: inherit;
		border-bottom: 1px solid var(--tool-line);
	}
	th {
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
	}
	/* the values: the mono face, figures in columns */
	.values td + td {
		font-family: var(--font-mono);
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
	}
</style>
