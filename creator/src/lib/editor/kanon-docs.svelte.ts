// The Kanon texts a page edits along with its own documents (research/KANON.md §5, #82 – the character page's K3 squares,
// research/CHARACTER-PAGE.md §3a): one ContentDoc per subject, opened when its square is first clicked, saved and
// published with the page (MetaPage `along`). The squares' state dots come from the doc once open, else from ONE
// content index request (the platform's rows: `kanon:<subject>:<l>`).
import { entities, type KanonRole } from '@scutoons/content';
import type { ContentIndexEntity } from '@scutoons/api';
import { contentIndex } from '$lib/api.ts';
import { ContentDoc } from '$lib/editor/doc.svelte.ts';

/** a text's state: not written in any language · a draft (unsaved or unpublished) · published */
export type KanonState = 'none' | 'draft' | 'published';

export class KanonDocs {
	/** subject segment → its doc (opened ones only) */
	docs = $state<Record<string, ContentDoc>>({});
	/** the stored Kanon rows (every subject, every language); null until loaded */
	private index = $state<ContentIndexEntity[] | null>(null);

	/** the platform's content index, the Kanon rows only (again after a save / publish: `refresh`) */
	async refresh(): Promise<void> {
		try {
			this.index = (await contentIndex('')).entities.filter((e) => e.kind === 'kanon');
		} catch {
			this.index = this.index ?? [];
		}
	}

	/** a subject's doc, created and loaded on first use, in the page's languages (the source first) */
	open(subject: string, languages: string[], source: string): ContentDoc {
		const known = this.docs[subject];
		if (known) return known;
		const doc = new ContentDoc(entities.kanon(subject), languages, source);
		this.docs[subject] = doc;
		void doc.load();
		return doc;
	}

	/** every opened doc (MetaPage `along`: saved when dirty, published with the page) */
	all(): ContentDoc[] {
		return Object.values(this.docs);
	}

	state(subject: string): KanonState {
		const doc = this.docs[subject];
		if (doc && !doc.loading) {
			if (doc.dirty || doc.unpublished.length) return 'draft';
			if (doc.stored.length) return 'published';
			return 'none';
		}
		const rows = (this.index ?? []).filter((e) => e.key.startsWith(`kanon:${subject}:`));
		if (!rows.length) return 'none';
		return rows.some((e) => e.latest !== e.published) ? 'draft' : 'published';
	}
}

/** the subject segment of a character page level */
export function characterSubject(character: string, persona: string | null, look: string | null): string {
	return ['character', character, persona, persona ? look : null].filter(Boolean).join('.');
}

/** the Kanon role of a character page level (kanon.ts) */
export const levelRole = (level: 'character' | 'persona' | 'look'): KanonRole => level;
