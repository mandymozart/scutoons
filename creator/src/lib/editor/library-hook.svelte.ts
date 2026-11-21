// An editor's side of the Library: it hands its open item's uses in (read from its document, a moment after the last
// edit) and forgets them when it closes; it takes the details' item action ("Set as cover").
import { library } from '$lib/editor/library.svelte.ts';
import { usesIn } from '$lib/editor/library-uses.ts';

/** the uses of `doc` by `address` for the Library, kept current while the editor is open */
export function shareUses(address: () => string | null, doc: () => unknown, toon: () => string | null = () => null) {
	$effect(() => {
		const a = address();
		const d = doc();
		const t = toon();
		if (!a || d == null) return;
		const snapshot = $state.snapshot(d);
		const timer = setTimeout(() => library.setUses(a, usesIn(a, snapshot, t)), 300);
		return () => clearTimeout(timer);
	});
	$effect(() => {
		const a = address();
		return () => {
			if (a) library.setUses(a, null);
		};
	});
}

/** the details' item action for `address` (`role`: `thumbnail` = Set as cover) */
export function onLibrarySet(role: string, address: () => string, apply: (path: string) => void) {
	$effect(() => {
		const listener = (e: Event) => {
			const d = (e as CustomEvent<{ role: string; path: string; item: string | null }>).detail;
			if (d.role === role && d.item === address()) apply(d.path);
		};
		window.addEventListener('sc-library-set', listener);
		return () => window.removeEventListener('sc-library-set', listener);
	});
}
