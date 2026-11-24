// The Toon tree's reading (lap2 C1): one toon – its seasons, each with the chapters its sequence lists – as the season and
// chapter documents, read for the rows' names and marks. Read again when asked (the panel asks on every page change), the
// last answer shown until the new one is in.
import { entities, previewText } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { loadToonLanguages, toonDoc } from '$lib/editor/languages.ts';
import { listLanguage } from '$lib/editor/text-language.svelte.ts';
import { seasonChapterNumbers } from '$lib/editor/visibility.ts';
import { fileLabel } from '$lib/labels.ts';
import { shared } from '$lib/utils/shared.ts';

export interface TreeSeason {
	number: number;
	doc: ContentDoc;
	chapters: number[];
}

/** a document's title in the list language (else its source's); '' when it has none */
export function titleOf(doc: ContentDoc | null | undefined): string {
	if (!doc || doc.loading) return '';
	return previewText(doc.get(doc.primary, ['title']), [listLanguage(doc.textLanguages, doc.textSource), doc.textSource]);
}

class ToonTree {
	toon = $state<string | null>(null);
	seasons = $state.raw<TreeSeason[]>([]);
	chapters = $state.raw<Record<number, ContentDoc>>({});
	#run = 0;

	/** reads `toon`'s seasons `numbers` (those without a document are left out) and their chapters */
	async read(toon: string, numbers: readonly number[]) {
		const run = ++this.#run;
		if (toon !== this.toon) {
			this.toon = toon;
			this.seasons = [];
			this.chapters = {};
		}
		try {
			const languages = await loadToonLanguages(toon);
			const seasons = await Promise.all(
				numbers.map(async (number) => {
					const doc = toonDoc(entities.season(toon, number), languages);
					await doc.load();
					return { number, doc, chapters: seasonChapterNumbers(doc) };
				})
			);
			const chapters: Record<number, ContentDoc> = {};
			await Promise.all(
				[...new Set(seasons.flatMap((s) => s.chapters))].map(async (n) => {
					const doc = toonDoc(entities.chapter(toon, n), languages);
					await doc.load();
					chapters[n] = doc;
				})
			);
			if (run !== this.#run) return;
			this.seasons = seasons.filter((s) => s.doc.present.length > 0);
			this.chapters = chapters;
		} catch {
			// unreadable (offline, no access): the tree keeps what it showed
		}
	}

	/** a season's row: "Season 1 · Awakening" */
	seasonLabel(s: TreeSeason): string {
		const title = titleOf(s.doc);
		const name = fileLabel(entities.season(this.toon ?? '', s.number));
		return title ? `${name} · ${title}` : name;
	}

	/** a chapter's row: its title, else its file's name */
	chapterLabel(n: number): string {
		return titleOf(this.chapters[n]) || fileLabel(entities.chapter(this.toon ?? '', n));
	}
}

export const toonTree = shared('creator-toon-tree', () => new ToonTree());
