import { getContext, setContext } from 'svelte';
import type { GroupScript } from './event-processor.ts';

/**
 * The group a slice belongs to (owner 2026-10-06: linked groups put several groups – and scripts – on one page): its
 * script and label (a slice event's divert goes into ITS group's script), its `scope` (the group's text boxes), and
 * `done` – a group above the `?label=` target of a chain counts as already read: no script, no slice triggers, its text
 * boxes shown as they are.
 */
export interface GroupScope extends GroupScript {
	done: boolean;
}

const KEY = Symbol('vn-group');

export function setGroupScope(scope: () => GroupScope): void {
	setContext(KEY, scope);
}

/** the slice's group (none outside a group, e.g. a preview) */
export function getGroupScope(): GroupScope | undefined {
	return getContext<(() => GroupScope) | undefined>(KEY)?.();
}
