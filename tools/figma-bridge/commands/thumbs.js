// thumbs – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by the storyboard scripts (matching pictures to shots).
// many nodes as small JPGs at one width – for matching pictures to shots: { ids, width } → [{ id, base64 } | { id, error }]
// Plugin-sandbox syntax: no `?.`, no object spread.
const out = [];
for (const id of args.ids ?? []) {
	try {
		const n = await figma.getNodeByIdAsync(id);
		if (!n || !('exportAsync' in n)) throw new Error('not exportable');
		const bytes = await n.exportAsync({ format: 'JPG', constraint: { type: 'WIDTH', value: args.width ?? 160 } });
		out.push({ id, base64: figma.base64Encode(bytes) });
	} catch (e) { out.push({ id, error: String(e && e.message ? e.message : e) }); }
}
return out;
