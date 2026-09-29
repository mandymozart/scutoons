// Rewrites the VN character references of chapter / scene files into the current form (owner, 2026-09-30):
//   joinScene / leaveScene  { name, position: { variant }, mood?, look?, persona? }  →  { character, persona, look?, mood?, variant? }
//   showMessage             { character: <old id> }                                   →  { character, persona }
// Former characters become personas (plf → catpope · human, cat → catpope · cat, mountain-tiger → catpope · mountaintiger,
// LEGACY_CHARACTER_IDS). Same meaning, new ids.
//
//   node --experimental-transform-types --no-warnings shared/content/scripts/migrate-scene-refs.ts [--dry-run] <file.json …>
//   (static files: viewer/static/api/catpope/{de,en}/chapter*.json and chapter*/scene-*.json)
//
// `transformSceneRefs(json)` is also used for the content store (see the report in agents/MEMORY.md "Characters").
import { readFileSync, writeFileSync } from 'node:fs';
import { REGULAR_PERSONA, normalizeSceneRef, resolveCharacterId } from '../src/index.ts';

/** Rewrites every joinScene / leaveScene / showMessage ref in a JSON tree (in place). Returns the number of refs changed. */
export function transformSceneRefs(node: unknown): number {
	let changed = 0;
	const visit = (value: unknown) => {
		if (Array.isArray(value)) return value.forEach(visit);
		if (!value || typeof value !== 'object') return;
		const obj = value as Record<string, any>;
		const params = obj.params;
		if ((obj.type === 'joinScene' || obj.type === 'leaveScene') && params && Array.isArray(params.characters)) {
			params.characters = params.characters.map((ref: any) => {
				const next = normalizeSceneRef(ref);
				if (!next) return ref;
				const out: Record<string, unknown> = { character: next.character, persona: next.persona };
				if (obj.type === 'joinScene') {
					if (next.look) out.look = next.look;
					if (next.mood) out.mood = next.mood;
					if (next.variant) out.variant = next.variant;
				}
				if (JSON.stringify(out) !== JSON.stringify(ref)) changed++;
				return out;
			});
		} else if (obj.type === 'showMessage' && params && typeof params.character === 'string' && params.character) {
			const next = resolveCharacterId(params.character, params.persona);
			if (next.character !== params.character || (params.persona || REGULAR_PERSONA) !== next.persona || params.persona === undefined) {
				const { character: _c, persona: _p, ...rest } = params;
				obj.params = { character: next.character, persona: next.persona, ...rest };
				changed++;
			}
		}
		for (const child of Object.values(obj)) visit(child);
	};
	visit(node);
	return changed;
}

// CLI
if (process.argv[1]?.replace(/\\/g, '/').endsWith('migrate-scene-refs.ts')) {
	const dry = process.argv.includes('--dry-run');
	for (const file of process.argv.slice(2).filter((a) => !a.startsWith('--'))) {
		const raw = readFileSync(file, 'utf8');
		const json = JSON.parse(raw);
		const n = transformSceneRefs(json);
		console.log(`${n ? 'rewrote' : 'unchanged'} ${file}${n ? ` (${n} refs)` : ''}`);
		if (n && !dry) writeFileSync(file, JSON.stringify(json, null, '\t') + '\n');
	}
}
