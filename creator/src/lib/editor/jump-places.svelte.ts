// The places a jump can lead to (owner 2026-10-06, "at label"): the toons, a toon's chapters and a chapter's group labels,
// loaded on demand for the jump target editor (toon → chapter → label) and for validation (`ValidateContext.targets`).
// One instance per chapter editor (Svelte context); the open chapter's own labels are passed live. A toon / chapter that
// doesn't load (no access, missing) stays unknown – validation says it can't check another toon's labels then.
import { getContext, setContext } from 'svelte';
import { contentIndex, getDraft } from '$lib/api.ts';
import { sceneEntityOfRef, type JumpTargets } from '@scutoons/content';
import { contextKey } from '$lib/utils/shared.ts';

const KEY = contextKey('jump-places');

/** a chapter's groups as the reader sees them: a ref group's label lives in its scene document */
async function chapterLabels(toon: string, number: number, keys: { chapter: string | null }): Promise<string[]> {
	if (!keys.chapter) return [];
	const doc: any = (await getDraft(keys.chapter)).content;
	const groups: any[] = Array.isArray(doc?.groups) ? doc.groups : [];
	const labels = await Promise.all(
		groups.map(async (g) => {
			const scene = typeof g?.ref?.src === 'string' ? sceneEntityOfRef(toon, g.ref.src) : null;
			if (!scene) return typeof g?.label === 'string' ? g.label : '';
			const s: any = await getDraft(scene)
				.then((r) => r.content)
				.catch(() => null);
			return typeof s?.label === 'string' ? s.label : typeof g?.label === 'string' ? g.label : '';
		})
	);
	return labels.filter((l) => !!l);
}

export class JumpPlaces {
	/** every toon (the creator's toon list: id + name); null = not loaded yet */
	toons = $state<{ id: string; name: string }[] | null>(null);
	/** toon → chapter number → its document key (neutral, else a language file); null = couldn't load */
	#chapters = $state<Record<string, Record<number, string> | null>>({});
	/** `toon/chapter` → group labels; null = couldn't load */
	#labels = $state<Record<string, string[] | null>>({});
	/** loads in flight (or done): one per toon / chapter */
	#pending = new Map<string, Promise<void>>();

	constructor(
		/** the open chapter's toon + number and its live labels */
		readonly toon: string,
		readonly chapter: number,
		private live: () => string[]
	) {}

	/** a toon's chapter numbers (loads them; undefined until known) */
	chapters(toon: string): number[] | undefined {
		const known = this.#chapters[toon];
		if (known === undefined) this.#loadChapters(toon);
		return known ? Object.keys(known).map(Number).sort((a, b) => a - b) : undefined;
	}

	/** a chapter's group labels (the open chapter: live; others load once); undefined until known */
	labels(toon: string, chapter: number): string[] | undefined {
		if (toon === this.toon && chapter === this.chapter) return this.live();
		const key = `${toon}/${chapter}`;
		const known = this.#labels[key];
		if (known === undefined) this.#loadLabels(toon, chapter);
		return known ?? undefined;
	}

	/** what validation checks targets against (`checkTargetPlace`) */
	targets(): JumpTargets {
		return { toon: this.toon, toons: this.toons?.map((t) => t.id), chapters: (t) => this.#known(t), labels: (t, n) => this.#knownLabels(t, n) };
	}

	/** read without starting a load (validation runs in a $derived – loads start from `prefetch`) */
	#known(toon: string): number[] | undefined {
		const known = this.#chapters[toon];
		return known ? Object.keys(known).map(Number) : undefined;
	}
	#knownLabels(toon: string, chapter: number): string[] | undefined {
		if (toon === this.toon && chapter === this.chapter) return this.live();
		return this.#labels[`${toon}/${chapter}`] ?? undefined;
	}

	/** load what the targets in `docs` name (chapters of their toons, their chapters' labels) */
	prefetch(docs: unknown[]) {
		const stack = [...docs];
		const seen = new Set<object>();
		while (stack.length) {
			const v = stack.pop();
			if (!v || typeof v !== 'object' || seen.has(v)) continue;
			seen.add(v);
			const o = v as Record<string, any>;
			if (!Array.isArray(v) && o.type === 'chapter' && 'value' in o && /^\d+$/.test(String(o.value))) {
				const toon = typeof o.toon === 'string' && o.toon ? o.toon : this.toon;
				this.chapters(toon);
				if (o.label || (Array.isArray(o.anyOf) && o.anyOf.length)) this.labels(toon, Number(o.value));
			}
			for (const key in o) stack.push(o[key]);
		}
	}

	#loadChapters(toon: string): Promise<void> {
		const key = `c:${toon}`;
		if (!this.#pending.has(key)) this.#pending.set(key, this.#fetchChapters(toon));
		return this.#pending.get(key)!;
	}

	async #fetchChapters(toon: string) {
		try {
			const index = (await contentIndex(toon)).entities.filter((e) => e.kind === 'chapter');
			const out: Record<number, string> = {};
			for (const e of [...index].sort((a, b) => Number(!!a.locale) - Number(!!b.locale) || a.key.localeCompare(b.key))) {
				const n = Number(e.key.split(':')[2]);
				if (Number.isFinite(n) && out[n] === undefined) out[n] = e.key;
			}
			this.#chapters[toon] = out;
		} catch {
			this.#chapters[toon] = null;
		}
	}

	#loadLabels(toon: string, chapter: number): Promise<void> {
		const key = `${toon}/${chapter}`;
		if (!this.#pending.has(key)) this.#pending.set(key, this.#fetchLabels(toon, chapter));
		return this.#pending.get(key)!;
	}

	async #fetchLabels(toon: string, chapter: number) {
		const key = `${toon}/${chapter}`;
		try {
			await this.#loadChapters(toon);
			const known = this.#chapters[toon];
			if (!known) throw new Error('no access');
			this.#labels[key] = await chapterLabels(toon, chapter, { chapter: known[chapter] ?? null });
		} catch {
			this.#labels[key] = null;
		}
	}
}

export function setJumpPlaces(places: JumpPlaces): JumpPlaces {
	return setContext(KEY, places);
}

/** the chapter editor's places (none outside a chapter editor: the target editor falls back to plain fields) */
export function getJumpPlaces(): JumpPlaces | undefined {
	return getContext<JumpPlaces | undefined>(KEY);
}
