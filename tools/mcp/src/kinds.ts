// read_kinds: the node-kind catalog (GET /api/kinds – deck/catalog's generated kinds.json plus the published modules).
export type KindDescriptor = { name: string; doc?: string; domain?: string; scope?: string; ports?: unknown[]; params?: unknown[] };
export type KindCatalog = { catalog_version: number; kinds: KindDescriptor[] };

/** the whole catalog as a short index (name, domain, doc), or the full descriptors of `names` (exact), or those matching `q` */
export function pickKinds(catalog: KindCatalog, names?: string[], q?: string) {
	const words = (q ?? '').toLowerCase().split(/\s+/).filter(Boolean);
	if (!names?.length && !words.length) {
		return { catalog_version: catalog.catalog_version, kinds: catalog.kinds.map((k) => ({ name: k.name, domain: k.domain, scope: k.scope, doc: k.doc })) };
	}
	const kinds = catalog.kinds.filter((k) => (names?.length ? names.includes(k.name) : true) && words.every((w) => JSON.stringify(k).toLowerCase().includes(w)));
	return { catalog_version: catalog.catalog_version, kinds };
}
