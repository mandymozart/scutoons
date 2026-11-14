// The conductor page's linked reader (#529, agents/research/SCENE-EVENTS.md §6.3): the link a conductor hands out comes
// from the core (deck-space `linked::reader_link`, relay-web's wasm) – the deck editor offers the same text; the linked
// readers are the core's view (deck-space `linked::Readers`, relay-web `readers()`, handed on by live-vote-link.ts). The
// places to pick are the reader's own: the toon's listed chapters (the library, as the toon page lists them) and a
// chapter's jump places (@scutoons/content `jumpLabels`, the creator's jump targets' list).

/** relay-web's link function (deck-space `linked::reader_link`; `label` '' = the chapter's start; `relay` the session's
 *  relay URL – the core names it in the link when it is not the hosted one, #530) */
export interface LinkModule {
	reader_link(base: string, toon: string, chapter: number, label: string, code: string, relay: string): string;
}

/** the conductor's view of the linked readers (relay-web `readers()`, deck-space `linked::Readers`) */
export interface LinkedReaders {
	/** client id → the name it said (null: none) */
	linked: Record<string, string | null>;
	/** the client that drives the session's story (null: nobody) */
	driver: string | null;
	/** the last story event heard: its type, the characters it names, the reader that sent it */
	last: { type: string; characters: string[]; from: string } | null;
}

/** what the conductor picked: a toon, a chapter of it, a label in it ('' = the chapter's start) */
export interface ReaderPlace {
	toon: string;
	chapter: number | null;
	label: string;
}

/** the address a linked reader opens for `place` in the session `code` on `relay` (the URL the conductor joined) – the
 *  core's text; '' until a toon and a chapter are picked */
export function readerLink(core: LinkModule, base: string, code: string, relay: string, place: ReaderPlace): string {
	if (!place.toon || place.chapter === null) return '';
	return core.reader_link(base, place.toon, place.chapter, place.label, code, relay);
}
