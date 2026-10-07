import type { ActionReturn } from 'svelte/action';

/**
 * THE selection model's live region (BACKLOG 10.40, agents/UI-CONCEPTS.md E.8, DESIGN.md "Selection").
 *
 * Every selectable area (a list, a table, the slice canvas + its inspector, a picker) is a region:
 * `use:region={'slices'}` (or `{ name, initial, modal }`). A `pointerdown` or `focusin` inside a region makes it the
 * live one – the innermost region around the target wins, so regions nest (the slice list inside the chapter tree).
 * The live region's element(s) carry `data-live`; tools.css then draws its `.sc-selected` items in the live look
 * (cyan tint, inset primary frame, dark blue text) and every other `.sc-selected` inverted. Pages ask
 * `liveRegion.is(name)` before a key acts (Delete / Backspace / Ctrl+X, arrows, Esc, later copy / paste).
 *
 * - `initial`: live when it mounts and nothing (mounted) is live yet – the page's main editor area;
 * - `modal`: live while mounted (sheets, the palette); unmounting returns live to the region before.
 * - A region that unmounts while live hands it back to the one live before it (if still mounted).
 */
export interface RegionOptions {
	/** the region's name (one per kind of selection on a page: 'tree', 'slices', 'slice', 'events', 'table' …) */
	name: string;
	/** live on mount when no other region is */
	initial?: boolean;
	/** live while mounted (a sheet, a dialog, the palette) */
	modal?: boolean;
}

class LiveRegion {
	/** the live region's name (null: none) */
	current = $state<string | null>(null);
	/** mounted regions: name → their elements */
	private nodes = new Map<string, Set<HTMLElement>>();
	/** names in the order they were live, the newest last (mounted ones only) */
	private history: string[] = [];

	/** is `name` the live region? (reactive) */
	is(name: string): boolean {
		return this.current === name;
	}

	/** makes `name` live */
	set(name: string | null) {
		if (name !== null) this.history = [...this.history.filter((n) => n !== name), name];
		if (this.current === name) return;
		this.current = name;
		this.apply();
	}

	/** @internal the action's bookkeeping */
	register(name: string, node: HTMLElement) {
		let set = this.nodes.get(name);
		if (!set) this.nodes.set(name, (set = new Set()));
		set.add(node);
		node.setAttribute('data-sc-region', name);
		if (this.current === name) node.setAttribute('data-live', '');
	}

	/** @internal */
	unregister(name: string, node: HTMLElement) {
		const set = this.nodes.get(name);
		set?.delete(node);
		node.removeAttribute('data-live');
		node.removeAttribute('data-sc-region');
		if (set && set.size) return;
		this.nodes.delete(name);
		this.history = this.history.filter((n) => n !== name);
		if (this.current === name) {
			this.current = this.history.at(-1) ?? null;
			this.apply();
		}
	}

	/** @internal is any region of that name mounted? */
	mounted(name: string | null): boolean {
		return name !== null && !!this.nodes.get(name)?.size;
	}

	private apply() {
		for (const [name, set] of this.nodes) for (const node of set) node.toggleAttribute('data-live', name === this.current);
	}
}

/** the one live region of the page (DESIGN.md "Selection") */
export const liveRegion = new LiveRegion();

/** the region action: `use:region={'slices'}` or `use:region={{ name: 'slice', initial: true }}`; null / '' = none */
export function region(node: HTMLElement, param: string | RegionOptions | null | undefined): ActionReturn<string | RegionOptions | null | undefined> {
	let options = normalize(param);

	/** the innermost region around the event's target takes it */
	function take(event: Event) {
		if (!options) return;
		const target = event.target as Element | null;
		if (target?.closest?.('[data-sc-region]') !== node) return;
		liveRegion.set(options.name);
	}
	function mount() {
		if (!options) return;
		liveRegion.register(options.name, node);
		if (options.modal || (options.initial && !liveRegion.mounted(liveRegion.current))) liveRegion.set(options.name);
	}
	node.addEventListener('pointerdown', take, true);
	node.addEventListener('focusin', take, true);
	mount();

	return {
		update(next) {
			const n = normalize(next);
			if (n?.name === options?.name && n?.modal === options?.modal) {
				options = n;
				return;
			}
			if (options) liveRegion.unregister(options.name, node);
			options = n;
			mount();
		},
		destroy() {
			node.removeEventListener('pointerdown', take, true);
			node.removeEventListener('focusin', take, true);
			if (options) liveRegion.unregister(options.name, node);
		}
	};
}

function normalize(param: string | RegionOptions | null | undefined): RegionOptions | null {
	if (!param) return null;
	return typeof param === 'string' ? { name: param } : param.name ? param : null;
}
