import { setLookImage, type AssetRequests, type CharacterFileSlot, type CharacterPersona, type MoodVocabulary } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { addAssetRequest, edit, editCharacter, editPersona, ensureMoods } from '$lib/editor/characters.ts';
import { closeRequests, setLookApose } from '$lib/editor/character-form.ts';

/** what a look's edits need (the character page's current persona / look) */
export type LookContext = {
	registry: ContentDoc;
	moodsDoc: ContentDoc;
	requestsDoc: ContentDoc;
	moods: MoodVocabulary;
	requests: AssetRequests | null;
	character: string;
	persona: CharacterPersona;
	lookId: string;
	languages: string[];
	/** the member's email for a new request (null = unknown) */
	email: string | null;
};

/**
 * The edits of one look (was character-look.svelte; research/CHARACTER-PAGE.md E splits the look over the stage's grid
 * and the inspector): a cell's image (closes its open request), a request per cell, a new mood (joins the vocabulary),
 * the A-pose, the look's own 3D model, default and remove. `ctx` is read on every call (the page's current look).
 */
export function lookEdits(ctx: () => LookContext) {
	function change(fn: (p: CharacterPersona) => void, key?: string) {
		const c = ctx();
		editPersona(c.registry, c.character, c.persona.id, fn, key ? `${c.character}:${c.persona.id}:${key}` : undefined);
	}
	const current = (p: CharacterPersona) => p.looks.find((l) => l.id === ctx().lookId);
	return {
		setImage(mood: string, view: string, path: string) {
			const c = ctx();
			change((p) => {
				const l = current(p);
				if (l) setLookImage(l, mood, view, path);
			}, `img-${c.lookId}-${mood}-${view}`);
			if (path) closeRequests(c.requestsDoc, c.requests, c.character, [{ kind: 'expression', persona: c.persona.id, look: c.lookId, mood, view, path } as CharacterFileSlot]);
		},
		request(mood: string, view: string, note: string) {
			const c = ctx();
			const existing = c.requests?.requests.find((r) => r.status === 'open' && r.character === c.character && r.persona === c.persona.id && r.look === c.lookId && r.mood === mood && r.view === view);
			if (existing) edit<AssetRequests>(c.requestsDoc, (f) => f.requests.forEach((r) => r.id === existing.id && (note.trim() ? (r.note = note.trim()) : delete r.note)));
			else addAssetRequest(c.requestsDoc, { character: c.character, persona: c.persona.id, look: c.lookId, mood, view }, note, c.email);
		},
		addMood(mood: string, label: string) {
			const c = ctx();
			if (!c.moods.moods.some((x) => x.id === mood)) {
				ensureMoods(c.moodsDoc);
				edit<MoodVocabulary>(c.moodsDoc, (v) => v.moods.push({ id: mood, label: Object.fromEntries(c.languages.map((l, i) => [l, i === 0 ? label : ''])), order: Math.max(0, ...v.moods.map((x) => x.order ?? 0)) + 1 }));
			}
			change((p) => {
				const l = current(p);
				if (l) l.images[mood] ??= {};
			});
		},
		/** the look's A-pose picked (closes its open request) or cleared */
		setApose(path: string | undefined) {
			const c = ctx();
			const target = c.lookId;
			editCharacter(c.registry, c.character, (ch) => setLookApose(ch, c.persona.id, target, path, c.languages), `${c.character}:${c.persona.id}:apose-${target}`);
			if (path) closeRequests(c.requestsDoc, c.requests, c.character, [{ kind: 'apose', persona: c.persona.id, look: target, newLook: false, path }]);
		},
		/** the look's own 3D model (a .glb); cleared = the persona's is used */
		setModel(path: string | null) {
			change((p) => {
				const l = current(p);
				if (!l) return;
				if (path) l.model = path;
				else delete l.model;
			}, `model-${ctx().lookId}`);
		},
		setLabel(label: Record<string, string> | undefined) {
			change((p) => {
				const l = current(p);
				if (!l) return;
				if (label) l.label = label;
				else delete l.label;
			}, `look-${ctx().lookId}`);
		},
		setDefault() {
			const id = ctx().lookId;
			change((p) =>
				p.looks.forEach((l) => {
					if (l.id === id) l.default = true;
					else delete l.default;
				})
			);
		},
		/** removes the look (never the last one); the caller confirms first */
		remove() {
			const target = ctx().lookId;
			change((p) => {
				if (p.looks.length < 2) return;
				p.looks = p.looks.filter((l) => l.id !== target);
				if (!p.looks.some((l) => l.default)) p.looks[0].default = true;
				p.transitions = p.transitions?.filter((t) => t.from !== target && t.to !== target);
				if (!p.transitions?.length) delete p.transitions;
			});
		}
	};
}
export type LookEdits = ReturnType<typeof lookEdits>;
