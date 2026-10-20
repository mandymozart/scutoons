// place – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by the storyboard scripts.
// copies of pictures beside a frame, at the height of the shot they belong to (the originals stay where they are):
// { page, group, items: [{ id, x, y, w }] } – x / y absolute on the page, w the copy's width (height keeps the
// aspect). The group (by name) is replaced on a re-run.
// Plugin-sandbox syntax: no `?.`, no object spread.
const page = figma.root.children.find((p) => p.name === args.page);
if (!page) throw new Error(`no page "${args.page}"`);
await page.loadAsync();
const old = page.children.find((n) => n.name === args.group);
if (old) old.remove();
const copies = [];
for (const it of args.items ?? []) {
	const n = await figma.getNodeByIdAsync(it.id);
	if (!n || !('clone' in n)) continue;
	const c = n.clone();
	page.appendChild(c);
	const s = it.w / c.width;
	if ('rescale' in c) c.rescale(s); else c.resize(it.w, c.height * s);
	c.x = it.x; c.y = it.y;
	c.name = `${it.shot ?? ''} · ${n.name}`;
	copies.push(c);
}
if (copies.length) { const g = figma.group(copies, page); g.name = args.group; }
return { placed: copies.length };
