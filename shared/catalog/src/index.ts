// @scutoons/catalog: the catalog editors of the tools (admin, creator) – agents/CATALOG-IN-CREATOR.md. The forms per kind,
// their fields, the kind definitions and the catalog problems (validateCatalog + accepted warnings). The app calls
// setupCatalog({ locale, locales, adminUrl }) once in its root layout (and setupMedia for the media slots).
export { setupCatalog, catalogLocale, catalogLocales, adminHref, type CatalogSetup } from './setup.ts';
export { m as catalogMessages, catalogMessage, CATALOG_MESSAGES, type CatalogMessageKey } from './messages.ts';
export * from './kinds.ts';
export * from './problems.ts';
export * from './import.ts';
export { default as AlbumForm } from './components/album-form.svelte';
export { default as HomeSceneForm } from './components/home-scene-form.svelte';
export { default as CodeTypeForm } from './components/code-type-form.svelte';
export { default as EditionForm } from './components/edition-form.svelte';
export { default as ItemForm } from './components/item-form.svelte';
export { default as MetaForm } from './components/meta-form.svelte';
export { default as PermissionForm } from './components/permission-form.svelte';
export { default as ProductForm } from './components/product-form.svelte';
export { default as RecipeForm } from './components/recipe-form.svelte';
export { default as ToonForm } from './components/toon-form.svelte';
export { default as CatalogField } from './components/catalog-field.svelte';
export { default as CatalogRows } from './components/catalog-rows.svelte';
export { default as CheckField } from './components/check-field.svelte';
export { default as JsonField } from './components/json-field.svelte';
export { default as LocaleText } from './components/locale-text.svelte';
export { default as RespawnField } from './components/respawn-field.svelte';
export { default as CatalogProblems } from './components/catalog-problems.svelte';
