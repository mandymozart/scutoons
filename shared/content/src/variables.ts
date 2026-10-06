/**
 * Story variable domains (owner 2026-10-06, agents/DATA.md "The save game"): one save game for every toon, but a name has
 * a domain so simple names (`completedQuest`) don't clash between toons:
 *
 *   as written            stored in the save game's `variables`
 *   `coins`               `<this toon>/coins`     this toon's variable (the default)
 *   `global.coins`        `coins`                 shared by every toon on SCUTOONS
 *   `toptoy.coins`        `toptoy/coins`          another toon's variable (read and write)
 *
 * The written form is relative to the toon the content is in; the storage key is absolute (the same in every toon), so
 * the save game, the flow index and the facts compare keys. Toon ids are lower-case letters, digits and `_` here (an id
 * with `-` or a leading digit can't be written in an expression); `global` and the expression namespaces (`reader`,
 * `map`, …) are never toon ids. Internal keys the reader writes itself (`picked:<slice>#<id>`) are no written names: they
 * belong to the toon (`<toon>/picked:…`).
 */

import { EXPR_NAMESPACES } from './expr/registry.ts';

/** the domain prefix of a global variable */
export const GLOBAL_VARIABLES = 'global';

/** a plain variable name: letters, digits, `_`, not starting with a digit */
export const VARIABLE_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** a name as written: `name`, `global.name` or `<toon>.name` */
export const WRITTEN_VARIABLE = /^(?:([a-z_][a-z0-9_]*)\.)?([A-Za-z_][A-Za-z0-9_]*)$/;

export type VariableDomain = 'toon' | 'global' | 'other';

export interface VariableRef {
	domain: VariableDomain;
	/** the toon the variable belongs to (none for a global one) */
	toon?: string;
	/** the plain name */
	name: string;
	/** the storage key */
	key: string;
}

/** the storage key of a written name, seen from `toon` */
export function variableKey(written: string, toon: string): string {
	return variableRef(written, toon).key;
}

/** a written name's domain, toon, plain name and storage key, seen from `toon` */
export function variableRef(written: string, toon: string): VariableRef {
	const m = WRITTEN_VARIABLE.exec(written);
	if (m && m[1] === GLOBAL_VARIABLES) return { domain: 'global', name: m[2], key: m[2] };
	if (m && m[1] && m[1] !== toon) return { domain: 'other', toon: m[1], name: m[2], key: `${m[1]}/${m[2]}` };
	const name = m ? m[2] : written;
	return { domain: 'toon', toon, name, key: `${toon}/${name}` };
}

/** a storage key as it is written in `toon`'s content (`catpope/x` → `x` in catpope, `catpope.x` elsewhere; `x` → `global.x`) */
export function writtenVariable(key: string, toon: string): string {
	const slash = key.indexOf('/');
	if (slash < 0) return `${GLOBAL_VARIABLES}.${key}`;
	const owner = key.slice(0, slash);
	const name = key.slice(slash + 1);
	return owner === toon ? name : `${owner}.${name}`;
}

/** the domain of a storage key (from `toon`'s view) */
export function keyDomain(key: string, toon: string): VariableDomain {
	const slash = key.indexOf('/');
	return slash < 0 ? 'global' : key.slice(0, slash) === toon ? 'toon' : 'other';
}

/** the plain name of a storage key (`toptoy/coins` → `coins`) */
export function keyName(key: string): string {
	return key.slice(key.indexOf('/') + 1);
}

/** whether a written name is valid where an author types one (setVariable, conditions, choices, askText) */
export function isWrittenVariable(written: string): boolean {
	return WRITTEN_VARIABLE.test(written);
}

const NAMESPACES = new Set(EXPR_NAMESPACES.map((ns) => ns.name));

/** whether a name an expression reads (its path) is a story variable: `coins`, `global.coins`, `toptoy.coins` – not
 *  a namespace member (`reader.name`, `map.total`) */
export function isStoryPath(path: readonly string[]): boolean {
	return path.length === 1 ? !NAMESPACES.has(path[0]) : path.length === 2 && !NAMESPACES.has(path[0]);
}
