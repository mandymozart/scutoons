// SCUTOONS Figma bridge – the plugin's main thread (Figma Plugin API). The UI (ui.html) holds the WebSocket to the
// local relay (tools/figma-bridge/relay.mjs on 127.0.0.1); commands arrive here, run against the open file, and the
// result goes back the same way. Runs only while the owner has the plugin open in Figma desktop – it acts as them.
// The core's commands: ping · pages · read · draft · run. Everything else is a module in tools/figma-bridge/commands/
// (export, region, sliceFrames, exportScales, thumbs, place …), loaded at runtime by `run` – new commands go there,
// not here: README "Command modules". The core keeps the helpers they share (lib: describe, nodeOrPage, sweep …).
// (agents/STORYBOARD.md §9 "SCUTOONS Figma bridge"). One command at a time (a queue here too); `ping` is answered at
// once, even while busy – { busy, version } for the runners (scripts/storyboard/lib.mjs figmaPreflight).
//
// !! EDITING THIS FILE (or ui.html) RESTARTS THE RUNNING PLUGIN IN FIGMA. With "Hot reload plugin" on (Figma desktop:
// Plugins → Development; seen 2026-10-20: every save → close code 1001, the new code connected 0.3 s later) it comes
// back by itself, but the command it was running is lost (its caller gets plugin_disconnected – figma() sends it again).
// With hot reload off, Figma closes it and the owner runs it again, once: Ctrl+Alt+P ("Run last plugin") or Plugins →
// Development → SCUTOONS Bridge. So batch the changes, bump VERSION with every change (the runners wait for the new
// code instead of finding out mid-run), and say so – never during an export run (#78, failures A / A' on 2026-10-18).
// Plugin-sandbox syntax: no `?.` and no object spread here (`??` and array spread work).

const VERSION = '2026-10-20.4';
const TEMP = 'scutoons-export (temporary)';

figma.showUI(__html__, { width: 260, height: 132, themeColors: true });

// <applyDraft> – shared with scripts/storyboard/figma-draft.mjs (it inlines this function for use_figma). Keep it
// self-contained: no closures over this file, Plugin API only.
async function applyDraft(spec) {
	let page = figma.root.children.find((p) => p.name === spec.page);
	if (!page) { page = figma.createPage(); page.name = spec.page; }
	await figma.setCurrentPageAsync(page);
	await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
	await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });
	// the lettering font of the presets (Bangers) if Figma has it, else a heavy Inter
	let lettering = { family: 'Bangers', style: 'Regular' };
	try { await figma.loadFontAsync(lettering); } catch { lettering = { family: 'Inter', style: 'Bold' }; }
	let frame = page.children.find((n) => n.name === spec.frame.name);
	if (!frame) {
		frame = figma.createFrame();
		frame.name = spec.frame.name;
		frame.x = spec.frame.x; frame.y = spec.frame.y ?? 0;
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
		let node;
		if (s.image) {
			// a picture (the render, or a Luma result): a rectangle with the image as its fill, at its size in the frame
			node = figma.createRectangle();
			node.resize(s.w, s.h);
			node.fills = [{ type: 'IMAGE', scaleMode: 'FILL', imageHash: figma.createImage(figma.base64Decode(s.image)).hash }];
		} else {
			node = figma.createNodeFromSvg(s.svg);
			if (s.scale && s.scale !== 1) node.rescale(s.scale);
		}
		node.name = s.id;
		node.x = s.x ?? 0; node.y = s.y;
		storyboard.appendChild(node);
		for (const d of s.texts ?? []) {
			const t = figma.createText();
			t.fontName = d.lettering ? lettering : { family: 'Inter', style: 'Regular' };
			t.fontSize = d.lettering ? d.size ?? 28 : 13;
			if (d.lettering) {
				// the Onomatopoeia / title preset: white letters, an ink outline
				t.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }];
				t.strokes = [{ type: 'SOLID', color: { r: 0, g: 0, b: 0 } }];
				t.strokeWeight = Math.max(1.5, (d.size ?? 28) * 0.1);
				t.strokeAlign = 'OUTSIDE';
				t.textAlignHorizontal = 'CENTER';
			}
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
	if (node.type === 'TEXT') {
		o.text = node.characters;
		// the lettering's look, for text boxes made from it (a mixed value – figma.mixed – is left out)
		const num = (v) => (typeof v === 'number' ? Math.round(v * 100) / 100 : undefined);
		const hex = (paints) => {
			const p = Array.isArray(paints) && [...paints].reverse().find((f) => f.type === 'SOLID' && f.visible !== false); // the top one shows
			return p ? '#' + [p.color.r, p.color.g, p.color.b].map((c) => Math.round(c * 255).toString(16).padStart(2, '0')).join('') : undefined;
		};
		Object.assign(o, { fontSize: num(node.fontSize), font: typeof node.fontName === 'object' ? node.fontName.family : undefined, align: typeof node.textAlignHorizontal === 'string' ? node.textAlignHorizontal : undefined, fill: hex(node.fills), stroke: hex(node.strokes), strokeWeight: num(node.strokeWeight) });
	}
	if ((node.type === 'TEXT' || node.type === 'FRAME') && node.absoluteBoundingBox) {
		const b = node.absoluteBoundingBox; // the rotated box on the page (x / y / w / h above are unrotated, parent-relative)
		o.abs = { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) };
	}
	if ('rotation' in node && Math.abs(node.rotation) > 0.01) o.rotation = Math.round(node.rotation * 10) / 10;
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

// Temporary frames (region's clipping box, always on the current page) are listed in the document's plugin data while
// they exist: a plugin closed in the middle of a region leaves its box at −200000 – removed on the next start (by id:
// its page loads on demand, no page is loaded whole for it) and after every region / export (the current page).
const TEMP_KEY = 'scutoons-temp';
function temps() {
	try { return JSON.parse(figma.root.getPluginData(TEMP_KEY) || '[]'); } catch (e) { return []; }
}
function setTemps(ids) {
	try { figma.root.setPluginData(TEMP_KEY, ids.length ? JSON.stringify(ids) : ''); } catch (e) {}
}
/** best effort, never throws: in dynamic-page mode a page that is not loaded throws on `children` */
function sweepPage(page) {
	try {
		const old = page.children.filter((c) => c.name === TEMP);
		for (const c of old) c.remove();
		return old.length;
	} catch (e) { return 0; }
}
const fileName = () => { try { return figma.root.name; } catch (e) { return null; } };
// what a layer shell takes over from its layer frame (region with layers) – everything that changes how its content looks
const LOOK = ['fills', 'strokes', 'strokeWeight', 'strokeAlign', 'effects', 'opacity', 'blendMode', 'clipsContent', 'cornerRadius'];
function copyLook(from, to) {
	for (const p of LOOK) {
		try { if (p in from && from[p] !== figma.mixed) to[p] = from[p]; } catch (e) {} // one that can't be set stays the shell's default (none)
	}
}
async function sweep() {
	let n = 0;
	for (const id of temps()) {
		try { const node = await figma.getNodeByIdAsync(id); if (node && node.name === TEMP) { node.remove(); n++; } } catch (e) {}
	}
	setTemps([]);
	return n + sweepPage(figma.currentPage);
}

// the queue: one command at a time (failure E – two page loads at once, then Figma gone); the UI forwards the state
// to the relay, which sends nothing new while the plugin is still busy (also with work from before a relay restart)
const queue = [];
let running = null; // { cmd, id, since }
const state = () => ({ busy: running ? { cmd: running.cmd, id: running.id, since: running.since } : null, queue: queue.length });
const tell = () => figma.ui.postMessage({ state: state() });
const errorText = (e) => String(e && e.message ? e.message : e);
// the leftovers of a closed plugin go first, before any command
const ready = sweep().then((n) => { if (n) figma.ui.postMessage({ note: `removed ${n} leftover export frame${n > 1 ? 's' : ''}` }); }, () => {});

async function pump() {
	if (running || !queue.length) return;
	const msg = queue.shift();
	running = { cmd: msg.cmd, id: msg.id, since: Date.now() };
	tell();
	try {
		await ready;
		const fn = commands[msg.cmd];
		if (!fn) throw new Error(`unknown command ${msg.cmd} (${Object.keys(commands).join(', ')})`);
		const result = await fn(msg.args || {});
		figma.ui.postMessage({ id: msg.id, ok: true, result });
	} catch (e) {
		figma.ui.postMessage({ id: msg.id, ok: false, error: errorText(e) });
	} finally {
		running = null;
		tell();
		pump();
	}
}

const commands = {
	async ping() {
		let user = null;
		try { user = figma.currentUser ? figma.currentUser.name : null; } catch {} // needs the 'currentuser' permission – not asked for
		const s = state();
		return { file: figma.root.name, page: figma.currentPage.name, user, editor: figma.editorType, version: VERSION, busy: s.busy, queue: s.queue, modules: Object.keys(modules).length, compiles, compiler };
	},
	async pages() {
		return figma.root.children.map((p) => ({ id: p.id, name: p.name }));
	},
	async read(args) {
		const node = await nodeOrPage(args);
		return describe(node, args.depth ?? 2);
	},
	async draft(args) {
		return applyDraft(args.spec);
	},
	/**
	 * a command module from tools/figma-bridge/commands/<name>.js (#93) – the relay reads the file per call and sends
	 * { name, hash, src, args }: compiled once per hash, then run as `async (args, lib) => { src }`. A new or edited
	 * module needs no plugin re-run and no relay restart.
	 */
	async run(msg) {
		if (!/^[a-z][\w-]*$/.test(String(msg.name))) throw new Error(`run: a module name, not "${msg.name}"`);
		let m = modules[msg.name];
		if (!m || m.hash !== msg.hash) {
			if (typeof msg.src !== 'string') throw new Error(`run: no source for module ${msg.name}`);
			m = modules[msg.name] = { hash: msg.hash, fn: compile(msg.name, msg.src) }; // an edited file replaces the old one
			compiles++;
		}
		return m.fn(msg.args || {}, lib);
	}
};

// ── command modules (#93) ──
const modules = {}; // name → { hash, fn }
let compiles = 0;
let compiler = null; // 'Function' | 'eval' – what this sandbox allows, found on the first module
/** a module's source → its async function; a syntax error throws (the caller gets plugin_error with the message) */
function compile(name, src) {
	const wrap = `(async function (args, lib) {\n${src}\n})\n//# sourceURL=commands/${name}.js`;
	const ways = {
		Function: () => new Function(`return ${wrap}`)(),
		eval: () => (0, eval)(wrap) // indirect: global scope, no access to this file's locals – modules get `lib`
	};
	let first = null;
	for (const way of compiler ? [compiler] : ['Function', 'eval']) {
		try { const fn = ways[way](); compiler = way; return fn; } catch (e) {
			if (e && e.name === 'SyntaxError') throw new Error(`module ${name}: ${errorText(e)}`); // the sandbox allows it – the source is wrong
			if (!first) first = e;
		}
	}
	throw new Error(`module ${name}: this plugin sandbox can't compile code (${errorText(first)})`);
}
/** what modules get as `lib`: the core's helpers and commands (figma itself is a global) */
const lib = { VERSION, TEMP, applyDraft, describe, nodeOrPage, sweep, sweepPage, temps, setTemps, copyLook, errorText, commands };

figma.ui.onmessage = (msg) => {
	if (msg && msg.uiReady) { figma.ui.postMessage({ hello: VERSION, file: fileName(), state: state() }); return; } // the UI (re)loaded
	if (!msg || !msg.id || !msg.cmd) return;
	if (msg.cmd === 'ping') { // past the queue: the health probe answers while a long command runs
		commands.ping().then((result) => figma.ui.postMessage({ id: msg.id, ok: true, result }), (e) => figma.ui.postMessage({ id: msg.id, ok: false, error: errorText(e) }));
		return;
	}
	queue.push(msg);
	tell();
	pump();
};
figma.ui.postMessage({ hello: VERSION, file: fileName(), state: state() });
