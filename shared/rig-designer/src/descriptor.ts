// A compiled module's descriptor as a read-only summary (RIG-EDITING.md §4: the SDK generates it, the compile extracts
// it). Language-neutral technical lines for the console; the Parameters tab (next) shows the full table.
import type { Descriptor, ModulePort } from './types.ts';

export interface DescriptorSummary {
	name: string;
	version: string;
	abi: number;
	paramCount: number;
	inputs: ModulePort[];
	outputs: ModulePort[];
}

export function summarize(d: Descriptor): DescriptorSummary {
	return {
		name: d.name,
		version: d.version,
		abi: d.abi,
		paramCount: d.params.length,
		inputs: d.ports.filter((p) => p.dir === 'in'),
		outputs: d.ports.filter((p) => p.dir === 'out')
	};
}

export const num = (n: number): string => String(Math.round(n * 1e4) / 1e4);

/** "walk 0.1.0 · ABI 2 · 3 params · 1 in · 2 out" */
export function headline(d: Descriptor): string {
	const s = summarize(d);
	return `${s.name} ${s.version} · ABI ${s.abi} · ${s.paramCount} params · ${s.inputs.length} in · ${s.outputs.length} out`;
}

/** one line per parameter and per port, for the console */
export function detailLines(d: Descriptor): string[] {
	return [
		...d.params.map((p) => `param ${p.name}  ${num(p.min)}…${num(p.max)}  = ${num(p.default)}`),
		...d.ports.map((p) => `${p.dir === 'in' ? 'in ' : 'out'} ${p.name}  ${p.type}${p.channels !== undefined ? ` ×${p.channels}` : ''}`)
	];
}
