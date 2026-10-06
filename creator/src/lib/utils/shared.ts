// Module-level state that must exist ONCE per page, even when the dev server serves its module twice (2026-10-06: after
// branch switches / merges the long-running Vite server handed out `expr-context.svelte.ts` under two `?t=` timestamps –
// the chapter editor wrote the story variables into one ExprContext, the builder read the other (empty) one; the Flow
// panel set its pictures under one `Symbol('flow-pictures')`, the boxes asked for the other – every picture an empty
// slot). Two copies of a module are two module scopes, so identity has to live outside them: the registry of
// `Symbol.for` and `globalThis`. Production bundles a module once – there this changes nothing.

/** a Svelte context key that is the same in every copy of the module that defines it */
export const contextKey = (name: string): symbol => Symbol.for(`scutoons:${name}`);

/** the page's one instance of `name` (made on first use; a second copy of the defining module gets the same one) */
export function shared<T>(name: string, make: () => T): T {
	const store = globalThis as unknown as Record<symbol, T | undefined>;
	const key = contextKey(name);
	return (store[key] ??= make());
}
