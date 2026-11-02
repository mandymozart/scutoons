// isolation.ts (#298, #282): which creator pages are cross-origin isolated (COOP same-origin + COEP credentialless) – the engine's
// (live-core's shared audio ring needs SharedArrayBuffer). Not the whole app: COEP blocks the Reader panel's viewer iframe
// (it sends no COEP itself) – measured in #298. The same paths in vite.config.ts
// (dev) and static/.htaccess (production); a document keeps its headers until a full load, so a client-side move into or out
// of these pages is a full load (isolation.svelte.ts).

/** the engine's routes: `/edit/…` (the editor, #282 / #245; #298's temporary `/engine` page is gone) */
export const ENGINE_PATH = /^\/edit(\/|$)/;

export const needsIsolation = (path: string): boolean => ENGINE_PATH.test(path);

/** The page needs headers it did not get (a client-side move into it): load it afresh – once; `tried` is the memory of that
 *  (a host that sends no headers must not loop). */
export const shouldReload = (isolated: boolean, tried: boolean): boolean => !isolated && !tried;

/** An isolated document moving on to `to`: a page that must not be isolated (the Reader panel's iframe is blocked in one) is
 *  a full load. */
export const mustLeave = (isolated: boolean, to: string): boolean => isolated && !needsIsolation(to);
