/**
 * What an address says about the creator's draft preview (lib/draft-preview.svelte.ts): `?draft=1` (the creator's docked
 * reader, "Open in the viewer") or `?preview=draft` (the older Preview links) switch it on, `?draft=0` off; null = the
 * address says nothing. Read when the module loads – before any load function – so the toon layout and the chapter
 * page never ask the library without `draft` while the root layout has not run yet.
 */
export function linkedPreview(search: string): boolean | null {
	const params = new URLSearchParams(search);
	const draft = params.get('draft');
	if (draft === '1' || draft === '0') return draft === '1';
	if (params.get('preview') === 'draft') return true;
	return null;
}
