// SCUTOONS Figma bridge – the plugin's main thread (Figma Plugin API). The UI (ui.html) holds the WebSocket to the
// local relay (tools/figma-bridge/relay.mjs on 127.0.0.1); commands arrive here, run against the open file, and the
// result goes back the same way. Runs only while the owner has the plugin open in Figma desktop – it acts as them.
// Commands: ping · pages · read · export · draft · region · sliceFrames · exportScales · thumbs · place · run (a module
// from tools/figma-bridge/commands/, loaded at runtime – new commands go there, not here: README "Command modules")
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

const VERSION = '2026-10-20.3';
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
	async export(args) {
		const node = await nodeOrPage(args);
		if (!('exportAsync' in node)) throw new Error(`${node.type} can't be exported`);
		const format = (args.format ?? 'PNG').toUpperCase();
		const settings = format === 'SVG' ? { format: 'SVG' } : { format, constraint: { type: 'SCALE', value: args.scale ?? 1 } };
		const bytes = await node.exportAsync(settings);
		sweepPage(figma.currentPage);
		return { name: node.name, format, bytes: bytes.length, base64: figma.base64Encode(bytes) };
	},
	async draft(args) {
		return applyDraft(args.spec);
	},
	/**
	 * a region of a frame as PNGs at several scales, some of its layers hidden – the production slices (figma-production.mjs):
	 * { frame, x, y, w, h, hide: ['text', 'guides'], hideIds: ['2231:5'], scales: [1, 2, 3], partial?, layers? } →
	 * [{ scale, base64 }] (hide = the frame's direct children by name, hideIds = layers at any depth by id). Works on a
	 * clone inside a clipping frame far off the canvas of the CURRENT page (always loaded – the draft's page may not be),
	 * removed afterwards – the draft itself is never touched.
	 * partial: clone only what reaches into the region (always for long drafts – a whole clone per slice was failure D);
	 * the box takes the frame's own fills (over white). layers: with partial, the frame's direct children are layer
	 * frames (a storyboard draft: storyboard / guides / text / layout) – per layer a shell with the layer's look (fills,
	 * strokes, effects, opacity, blend mode, clipping) at the layer's place, holding clones of only ITS children that
	 * reach into the region, each at its own place in the layer – not the whole layer. A layer holding a mask is cloned
	 * whole (a mask alone would cover the rest).
	 */
	async region(args) {
		const f = await figma.getNodeByIdAsync(args.frame);
		if (!f || !('clone' in f)) throw new Error(`no frame ${args.frame}`);
		const page = figma.currentPage;
		const box = figma.createFrame();
		box.name = TEMP;
		const boxId = box.id;
		setTemps(temps().concat(boxId));
		box.resize(args.w, args.h);
		box.clipsContent = true;
		// white under everything (a transparent PNG turns black in the WebP); partial: the frame's own fills on top – the
		// whole clone below brings them itself
		box.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }].concat(args.partial && Array.isArray(f.fills) ? f.fills : []);
		page.appendChild(box);
		box.x = -200000; box.y = -200000;
		// hideIds: layers hidden at any depth (lettering nested in groups) – the clone keeps the original's child order,
		// so original and clone are walked side by side
		const ids = new Set(args.hideIds ?? []);
		const hideIn = (o, c) => {
			if (!ids.size || !('children' in o) || !('children' in c)) return;
			o.children.forEach((k, i) => { const ck = c.children[i]; if (!ck) return; if (ids.has(k.id)) ck.visible = false; else hideIn(k, ck); });
		};
		try {
			if (args.partial && 'children' in f) {
				// a huge hand-made draft: clone only the frame's direct children that reach into the region (their
				// own positions stay – a child's x / y are relative to the frame), not the whole frame each time
				const fb = f.absoluteBoundingBox;
				// by the drawn box on the page: a rotated layer reaches far from its x / y (blood vessels turned 146°)
				const reaches = (n, dx, dy) => {
					const b = n.absoluteBoundingBox;
					const nx = b && fb ? b.x - fb.x : dx + n.x, ny = b && fb ? b.y - fb.y : dy + n.y, nw = b ? b.width : n.width, nh = b ? b.height : n.height;
					return !(nx >= args.x + args.w || nx + nw <= args.x || ny >= args.y + args.h || ny + nh <= args.y);
				};
				for (const n of f.children) {
					if ((args.hide ?? []).includes(n.name) || n.visible === false || ids.has(n.id) || !reaches(n, 0, 0)) continue;
					// a layer frame (not a group, not one holding a mask): a shell in its place with its look, its children
					// that reach into the region inside, each with its own transform in the layer (rotation included)
					if (args.layers && n.type === 'FRAME' && !n.children.some((k) => k.isMask)) {
						const shell = figma.createFrame();
						box.appendChild(shell);
						shell.name = n.name;
						shell.fills = []; shell.clipsContent = false; // a new frame is white and clips – the layer's own look below
						copyLook(n, shell);
						shell.resize(Math.max(0.01, n.width), Math.max(0.01, n.height));
						const t = n.relativeTransform;
						shell.relativeTransform = [[t[0][0], t[0][1], t[0][2] - args.x], [t[1][0], t[1][1], t[1][2] - args.y]];
						for (const k of n.children) {
							if (k.visible === false || ids.has(k.id) || !reaches(k, n.x, n.y)) continue;
							const c = k.clone();
							shell.appendChild(c);
							c.relativeTransform = k.relativeTransform;
							hideIn(k, c);
						}
						continue;
					}
					const c = n.clone();
					box.appendChild(c);
					c.x = n.x - args.x; c.y = n.y - args.y;
					hideIn(n, c);
				}
				const out = [];
				for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await box.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
				return out;
			}
			const c = f.clone();
			box.appendChild(c);
			c.x = -args.x; c.y = -args.y;
			if (args.unclip && 'clipsContent' in c) c.clipsContent = false; // a panorama strip reaches past the frame's width
			if ('children' in c) for (const l of c.children) if ((args.hide ?? []).includes(l.name)) l.visible = false;
			hideIn(f, c);
			const out = [];
			for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await box.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
			return out;
		} finally {
			// never throws: an exception here would replace the export's result
			try { box.remove(); setTemps(temps().filter((id) => id !== boxId)); } catch (e) {} // still listed: the next sweep takes it
			sweepPage(page);
		}
	},
	/**
	 * production slice frames from a draft – the owner's way ("ich ziehe immer die ebenen auf einen slice die auf der
	 * selben hoehe im slice active werden … dannach loesche ich die textebenen"): per slice a frame (name, w × h,
	 * clipping) at { px, py } on the page, holding copies of every node of the draft's layers (minus `skip`) that reaches
	 * into the slice, placed as in the draft, unlocked (owner: locked layers must be unlocked before copying); hidden
	 * nodes stay hidden. { frame, skip: ['text', 'guides'], slices: [{ name, x, y, w, h, px, py }] } – frames of the same
	 * name are replaced.
	 */
	async sliceFrames(args) {
		const f = await figma.getNodeByIdAsync(args.frame);
		if (!f || !('children' in f)) throw new Error(`no frame ${args.frame}`);
		const page = f.parent;
		const nodes = [];
		for (const layer of f.children) {
			if ((args.skip ?? []).includes(layer.name)) continue;
			const kids = 'children' in layer && layer.type !== 'INSTANCE' ? layer.children : [layer];
			for (const n of kids) nodes.push({ n, x: (layer === n ? 0 : layer.x) + n.x, y: (layer === n ? 0 : layer.y) + n.y, w: n.width, h: n.height });
		}
		const out = [];
		for (const s of args.slices ?? []) {
			for (const old of page.children.filter((c) => c.name === s.name)) old.remove();
			const fr = figma.createFrame();
			fr.name = s.name;
			fr.resize(s.w, s.h);
			fr.clipsContent = true;
			fr.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }];
			page.appendChild(fr);
			fr.x = s.px; fr.y = s.py;
			let count = 0;
			for (const it of nodes) {
				if (it.x >= s.x + s.w || it.x + it.w <= s.x || it.y >= s.y + s.h || it.y + it.h <= s.y) continue;
				const c = it.n.clone();
				fr.appendChild(c);
				c.x = it.x - s.x; c.y = it.y - s.y;
				if ('locked' in c) c.locked = false;
				count++;
			}
			out.push({ name: s.name, id: fr.id, layers: count });
		}
		return out;
	},
	/** a node (a slice frame) as PNGs at several scales: { node, scales } → [{ scale, base64 }] */
	async exportScales(args) {
		const n = await figma.getNodeByIdAsync(args.node);
		if (!n || !('exportAsync' in n)) throw new Error(`no node ${args.node}`);
		const out = [];
		for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await n.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
		sweepPage(figma.currentPage);
		return out;
	},
	/** many nodes as small JPGs at one width – for matching pictures to shots: { ids, width } → [{ id, base64 } | { id, error }] */
	async thumbs(args) {
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
	},
	/**
	 * copies of pictures beside a frame, at the height of the shot they belong to (the originals stay where they are):
	 * { page, group, items: [{ id, x, y, w }] } – x / y absolute on the page, w the copy's width (height keeps the
	 * aspect). The group (by name) is replaced on a re-run.
	 */
	async place(args) {
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
