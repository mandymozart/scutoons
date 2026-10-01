import { isPrologue } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

/** "Kapitel 2" / "Chapter 2", and for chapter 0 (the season's prologue) "Prolog" / "Prologue" */
export function chapterName(number: number | string): string {
	return isPrologue(Number(number)) && String(number).trim() !== '' ? m['chapter.prologue']() : m['common.chapter']({ number });
}
