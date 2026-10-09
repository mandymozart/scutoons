// SCUTOONS Figma bridge – the plugin's main thread (Figma Plugin API). The UI (ui.html) holds the WebSocket to the
// local relay (tools/figma-bridge/relay.mjs on 127.0.0.1); commands arrive here, run against the open file, and the
// result goes back the same way. Runs only while the owner has the plugin open in Figma desktop – it acts as them.
// Commands: ping · pages · read · export · draft (agents/STORYBOARD.md §9 "SCUTOONS Figma bridge").

figma.showUI(__html__, { width: 260, height: 132, themeColors: true });

// <applyDraft> – shared with scripts/storyboard/figma-draft.mjs (it inlines this function for use_figma). Keep it
// self-contained: no closures over this file, Plugin API only.
async function applyDraft(spec) {
	let page = figma.root.children.find((p) => p.name === spec.page);
	if (!page) { page = figma.createPage(); page.name = spec.page; }
	await figma.setCurrentPageAsync(page);
	await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
	await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });
	let frame = page.children.find((n) => n.name === spec.frame.name);
	if (!frame) {
		frame = figma.createFrame();
		frame.name = spec.frame.name;
		frame.x = spec.frame.x; frame.y = 0;
		frame.clipsContent = true;
		frame.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }];
	}
	frame.resize(spec.frame.w, Math.max(1, spec.frame.h));
	const layer = (name, locked) => {
		let f = frame.children.find((n) => n.name === name);
		if (!f) { f = figma.createFrame(); f.name = name; f.fills = []; f.clipsContent = false; frame.appendChild(f); }
		f.x = 0; f.y = 0; f.resize(spec.frame.w, Math.max(1, spec.frame.h)); f.locked = locked;
		return f;
	};
	const storyboard = layer('storyboard', true);
	const guides = layer('guides', true);
	const text = layer('text', false);
	const layout = layer('layout', false);
	// bottom → top: storyboard, guides, text, layout (the owner's work stays on top and is never touched)
	frame.appendChild(storyboard); frame.appendChild(guides); frame.appendChild(text); frame.appendChild(layout);
	if (spec.guides) {
		for (const n of [...guides.children]) n.remove();
		for (const g of spec.guides) {
			const r = figma.createRectangle();
			r.name = `${g.kind} ${g.y}`; r.x = 0; r.y = g.y; r.resize(spec.frame.w, 1);
			r.fills = [{ type: 'SOLID', color: g.kind === 'screen' ? { r: 0.9, g: 0.2, b: 0.2 } : { r: 0.2, g: 0.4, b: 0.9 }, opacity: 0.5 }];
			guides.appendChild(r);
		}
		guides.visible = false;
	}
	if (spec.rail) {
		const railName = `${spec.frame.name} · rail`;
		const old = page.children.find((n) => n.name === railName);
		if (old) old.remove();
		const nodes = spec.rail.map((r) => {
			const t = figma.createText();
			t.fontName = { family: 'Inter', style: 'Regular' }; t.fontSize = 11; t.characters = r.label;
			t.x = frame.x + spec.frame.w + 12; t.y = r.y; page.appendChild(t);
			return t;
		});
		if (nodes.length) { const g = figma.group(nodes, page); g.name = railName; g.locked = true; }
	}
	for (const s of spec.shots ?? []) {
		for (const n of [...storyboard.children, ...text.children]) if (n.name === s.id || n.name.startsWith(`${s.id} ·`)) n.remove();
		const node = figma.createNodeFromSvg(s.svg);
		node.name = s.id;
		if (s.scale && s.scale !== 1) node.rescale(s.scale);
		node.x = s.x ?? 0; node.y = s.y;
		storyboard.appendChild(node);
		for (const d of s.texts ?? []) {
			const t = figma.createText();
			t.fontName = { family: 'Inter', style: d.lettering ? 'Bold' : 'Regular' };
			t.fontSize = d.lettering ? 28 : 13;
			t.characters = d.text;
			t.textAutoResize = 'HEIGHT';
			t.resize(Math.max(40, d.w), t.height);
			t.x = d.x; t.y = d.y; t.name = `${s.id} · ${d.variant}`;
			text.appendChild(t);
		}
	}
	return { page: page.id, frame: frame.id, shots: storyboard.children.length, texts: text.children.length, layout: layout.children.length };
}
// </applyDraft>

function describe(node, depth) {
	const o = { id: node.id, name: node.name, type: node.type };
	if ('x' in node) Object.assign(o, { x: Math.round(node.x), y: Math.round(node.y), w: Math.round(node.width), h: Math.round(node.height) });
	if (node.type === 'TEXT') o.text = node.characters;
	if ('fills' in node && Array.isArray(node.fills)) {
		const img = node.fills.find((f) => f.type === 'IMAGE');
		if (img) o.image = img.imageHash;
	}
	if ('locked' in node && node.locked) o.locked = true;
	if ('visible' in node && !node.visible) o.hidden = true;
	if ('children' in node) {
		o.childCount = node.children.length;
		if (depth > 0) o.children = node.children.map((c) => describe(c, depth - 1));
	}
	return o;
}

async function nodeOrPage(args) {
	if (args.nodeId) {
		const n = await figma.getNodeByIdAsync(args.nodeId);
		if (!n) throw new Error(`no node ${args.nodeId}`);
		return n;
	}
	if (args.page) {
		const p = figma.root.children.find((x) => x.name === args.page || x.id === args.page);
		if (!p) throw new Error(`no page "${args.page}"`);
		await p.loadAsync();
		return p;
	}
	return figma.currentPage;
}

const commands = {
	async ping() {
		return { file: figma.root.name, page: figma.currentPage.name, user: figma.currentUser ? figma.currentUser.name : null, editor: figma.editorType };
	},
	async pages() {
		return figma.root.children.map((p) => ({ id: p.id, name: p.name }));
	},
	async read(args) {
		const node = await nodeOrPage(args);
		return describe(node, args.depth ?? 2);
	},
	async export(args) {
		const node = await nodeOrPage(args);
		if (!('exportAsync' in node)) throw new Error(`${node.type} can't be exported`);
		const format = (args.format ?? 'PNG').toUpperCase();
		const settings = format === 'SVG' ? { format: 'SVG' } : { format, constraint: { type: 'SCALE', value: args.scale ?? 1 } };
		const bytes = await node.exportAsync(settings);
		return { name: node.name, format, bytes: bytes.length, base64: figma.base64Encode(bytes) };
	},
	async draft(args) {
		return applyDraft(args.spec);
	}
};

figma.ui.onmessage = async (msg) => {
	if (!msg || !msg.id || !msg.cmd) return;
	try {
		const fn = commands[msg.cmd];
		if (!fn) throw new Error(`unknown command ${msg.cmd} (${Object.keys(commands).join(', ')})`);
		const result = await fn(msg.args ?? {});
		figma.ui.postMessage({ id: msg.id, ok: true, result });
	} catch (e) {
		figma.ui.postMessage({ id: msg.id, ok: false, error: String(e && e.message ? e.message : e) });
	}
};
