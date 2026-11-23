import type { Spores } from '@scutoons/api';
import { pluginCodeUses } from '../osct/editor/src/lib/logic/plugin-code';
import { levelName } from '../osct/editor/src/lib/logic/navigation';
import { levelsOf, project, type LogicContent, type LogicScene } from '../osct/editor/src/lib/logic/project';
import { coreRegistry } from '../osct/editor/src/lib/logic/registry';
import type { Context, GraphEdge, GraphNode, Home, PortSpec, Signal } from '../osct/editor/src/lib/logic/types';
import type { EntityData } from '../osct/engine/src/types';
import { logicRules } from '../osct/shared/types/logic';
import type * as api from './network-api';

type Obj = Record<string, unknown>;
const obj = (v: unknown): Obj | null => (v && typeof v === 'object' && !Array.isArray(v) ? (v as Obj) : null);
const list = (v: unknown): unknown[] => (Array.isArray(v) ? v : []);
const str = (v: unknown): string => (typeof v === 'string' ? v : '');

/** OSCT's signals in the deck's one table (#763 contract): `value` is `control`, `node` an `address`, `media` a `picture` */
const SIGNAL: Record<Signal, Spores.Signal> = { event: 'event', value: 'control', node: 'address', scene: 'scene', address: 'address', far: 'far', media: 'picture' };

/** The experience document as OSCT's logic model reads it: the scenes with their targets' trees, rules, groups, plugins */
function logicContent(document: unknown): { content: LogicContent; id: string; configuration: Obj } {
	const doc = obj(document) ?? {};
	const configuration = obj(doc.configuration) ?? {};
	const shared = obj(configuration.entities) ?? {};
	const entries = list(configuration.entries).map(obj).filter((e): e is Obj => !!e);
	const entityOf = (target: Obj): EntityData | undefined => {
		const entity = obj(target.entity);
		const ref = str(entity?.ref);
		return ((ref ? obj(shared[ref]) : entity) ?? undefined) as unknown as EntityData | undefined;
	};
	const scenes: LogicScene[] = list(configuration.scenes)
		.map(obj)
		.filter((s): s is Obj => !!s)
		.map((s) => {
			const own = entries.filter((e) => str(e.scene) === str(s.id));
			const logic = obj(s.logic);
			return {
				id: str(s.id),
				label: str(s.label) || str(s.id),
				...(str(s.group) ? { group: str(s.group) } : {}),
				targets: own.flatMap((e) => {
					const target = obj(e.target);
					return target && str(target.id) ? [{ id: str(target.id), entity: entityOf(target) }] : [];
				}),
				entries: own.map((e) => ({ id: str(e.id), title: str(e.title), ...(str(e.body) ? { body: str(e.body) } : {}) })),
				...(logic ? { rules: logicRules(logic), ...(obj(logic.canvas) ? { canvas: logic.canvas as never } : {}) } : {})
			};
		});
	const plugins = list(doc.plugins).map(obj).filter((p): p is Obj => !!p && !!str(p.id));
	const experience = obj(configuration.experience) ?? {};
	const logic = obj(experience.logic);
	const content: LogicContent = {
		scenes,
		groups: list(configuration.groups) as never,
		plugins: plugins.map((p) => ({ id: str(p.id), api: str(p.api), src: `${str(p.id)}.js` })),
		code: Object.fromEntries(plugins.map((p) => [str(p.id), pluginCodeUses(str(p.code))])),
		...(logic ? { rules: logicRules(logic), ...(obj(logic.canvas) ? { canvas: logic.canvas as never } : {}) } : {}),
		...(obj(experience.variables) ? { variables: experience.variables as never } : {})
	};
	return { content, id: str(doc.id), configuration };
}

/** a level's address in the one grammar (deck/space/src/rig/address.rs: `kind:id` places joined by `/`) */
function addressOf(root: string, context: Context): string {
	switch (context.level) {
		case 'experience':
			return root;
		case 'group':
			return `${root}/group:${context.group}`;
		case 'scene':
			return `${root}/scene:${context.scene}`;
		case 'entity':
			return `${root}/scene:${context.scene}/target:${context.target}${context.path.length ? `/node:${context.path.join('.')}` : ''}`;
	}
}

/** the level an address names; anything else: the whole experience */
function contextOf(root: string, address: string): Context {
	const rest = address.startsWith(`${root}/`) ? address.slice(root.length + 1).split('/') : [];
	const part = (kind: string) => rest.find((p) => p.startsWith(`${kind}:`))?.slice(kind.length + 1) ?? null;
	const scene = part('scene');
	const target = part('target');
	const group = part('group');
	const node = part('node');
	if (scene && target) return { level: 'entity', scene, target, path: node ? node.split('.').map(Number).filter((n) => Number.isInteger(n) && n >= 0) : [] };
	if (scene) return { level: 'scene', scene };
	if (group) return { level: 'group', group };
	return { level: 'experience' };
}

/** a JSON pointer segment (RFC 6901) */
const seg = (part: string | number) => String(part).replaceAll('~', '~0').replaceAll('/', '~1');

/** where a rule's part is written in the experience document: a JSON pointer */
function pointerOf(configuration: Obj, home: Home): string | null {
	const tail = home.part === 'do' && home.index !== undefined ? `/do/${home.index}` : home.part === 'on' ? '/on' : home.part === 'when' ? '/when' : '';
	if (home.doc === 'experience') return `/configuration/experience/logic/rules/${home.rule}${tail}`;
	if (home.doc === 'scene') {
		const index = list(configuration.scenes).findIndex((s) => str(obj(s)?.id) === home.scene);
		return index < 0 ? null : `/configuration/scenes/${index}/logic/rules/${home.rule}${tail}`;
	}
	const entries = list(configuration.entries).map(obj);
	const index = entries.findIndex((e) => str(obj(e?.target)?.id) === home.target);
	const target = obj(entries[index]?.target);
	if (index < 0 || !target) return null;
	const ref = str(obj(target.entity)?.ref);
	const root = ref ? `/configuration/entities/${seg(ref)}` : `/configuration/entries/${index}/target/entity`;
	return `${root}${home.path.map((i) => `/children/${i}`).join('')}/components/${home.component}/rules/${home.rule}${tail}`;
}

const port = (p: PortSpec): Spores.Port => ({
	id: p.id,
	label: p.label ?? '',
	signal: SIGNAL[p.signal],
	dir: p.dir,
	...(p.many ? { many: true } : {}),
	...(p.hidden ? { hidden: true } : {})
});

export const projectXr: typeof api.projectXr = (document, address) => {
	const { content, id, configuration } = logicContent(document);
	const root = `experience:${id}`;
	const context = contextOf(root, address);
	const view = project(content, context);
	const doc = root;
	const nodeAddress = (n: GraphNode): string => {
		const pick = n.pick;
		if (pick?.kind === 'node') {
			const scene = content.scenes.find((s) => s.targets.some((t) => t.id === pick.target))?.id ?? '';
			return addressOf(root, { level: 'entity', scene, target: pick.target, path: pick.path });
		}
		if (pick?.kind === 'plugin') return `${root}/plugin:${pick.plugin}`;
		if (pick?.kind === 'scene') return `${root}/scene:${pick.scene}`;
		if (pick?.kind === 'group') return `${root}/group:${pick.group}`;
		return `${addressOf(root, context)}/box:${n.id}`;
	};
	const spores: Spores.Spore[] = view.nodes.map((n) => ({
		id: n.id,
		kind: n.kind,
		family: n.far ? 'far' : (coreRegistry.get(n.kind)?.family ?? 'plugin'),
		label: n.label,
		...(n.detail ? { detail: n.detail } : {}),
		address: nodeAddress(n),
		driver: 'xr',
		ports: n.ports.map(port),
		...(n.enter ? { enter: addressOf(root, n.enter) } : {}),
		...(n.preview ? { picture: n.preview } : {})
	}));
	// the experience's groups frame their scenes on the canvas – the one place a spore's group is kept (#763); the group
	// stays a spore too, the box its level is entered by (OSCT names the frame `group:<id>`, the canvas group `<id>`)
	const framed = new Map<string, string[]>();
	for (const n of view.nodes) if (n.group) framed.set(n.group.replace(/^group:/, ''), [...(framed.get(n.group.replace(/^group:/, '')) ?? []), n.id]);
	const groups: Spores.Group[] = [...framed].map(([group, members]) => ({
		id: group,
		label: str(list(configuration.groups).map(obj).find((g) => str(g?.id) === group)?.label) || group,
		spores: members
	}));
	const thread = (e: GraphEdge): Spores.Thread => {
		const path = e.home ? pointerOf(configuration, e.home) : null;
		return {
			id: e.id,
			from: { spore: e.from[0], port: e.from[1] },
			to: { spore: e.to[0], port: e.to[1] },
			signal: SIGNAL[e.signal],
			...(e.label ? { label: e.label } : {}),
			...(path ? { home: { doc, path } } : {}),
			crosses: 'same'
		};
	};
	const threads = view.edges.map(thread);
	const ids = new Set(spores.map((n) => n.id));
	// a thread that leaves the level: its outside end becomes a far spore named by where it goes
	const stubs: Spores.Stub[] = view.stubs.map((s, i) => {
		const far = addressOf(root, s.far);
		const farId = `far-${i}`;
		const inside = s.side === 'out' ? s.edge.from : s.edge.to;
		const signal = SIGNAL[s.edge.signal];
		spores.push({ id: farId, kind: 'stub', family: 'far', label: s.label, address: far, driver: 'xr', ports: [{ id: s.side === 'out' ? 'in' : 'out', label: '', signal: 'far', dir: s.side === 'out' ? 'in' : 'out' }] });
		const id = ids.has(s.edge.id) ? `${s.edge.id}-stub-${i}` : s.edge.id;
		const path = s.edge.home ? pointerOf(configuration, s.edge.home) : null;
		threads.push({
			id,
			from: s.side === 'out' ? { spore: inside[0], port: inside[1] } : { spore: farId, port: 'out' },
			to: s.side === 'out' ? { spore: farId, port: 'in' } : { spore: inside[0], port: inside[1] },
			signal,
			...(s.edge.label ? { label: s.edge.label } : {}),
			...(path ? { home: { doc, path } } : {}),
			crosses: s.far.level === 'scene' || s.far.level === 'entity' ? (s.edge.signal === 'scene' ? 'navigation' : 'savegame') : 'same'
		});
		return { thread: id, side: s.side, far, label: s.label };
	});
	return {
		scope: 'xr',
		context: addressOf(root, context),
		levels: levelsOf(content, context).map((l) => ({ address: addressOf(root, l), label: l.level === 'experience' ? str(obj(configuration.experience)?.title) || id : levelName(content, l) })),
		spores,
		threads,
		stubs,
		problems: view.problems.map((p) => ({ at: p.node ?? '', severity: p.severity, text: p.short ?? p.message })),
		...(groups.length ? { canvas: { groups } } : {})
	};
};
