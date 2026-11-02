// What the designer needs from the app, provided ONCE (like @scutoons/catalog's setupCatalog): the UI locale. A module
// setting, not a Svelte context – messages.ts is called outside components too. No paraglide inside the package.
export interface RigDesignerSetup {
	/** the app's UI locale (paraglide `getLocale`); read on every render */
	locale: () => string;
}

let current: RigDesignerSetup | null = null;

export function setupRigDesigner(setup: RigDesignerSetup): void {
	current = setup;
}

/** "de" or "en"; English until the app has called setupRigDesigner (tests, a story) */
export const designerLocale = (): string => current?.locale() ?? 'en';
