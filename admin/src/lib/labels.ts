import { entityLabel } from '@scutoons/ui';
import type { EventName } from '@scutoons/api';
import { m } from '$lib/paraglide/messages.js';
import { number } from '$lib/format.ts';

const EVENTS: Record<EventName, () => string> = {
	scan: () => m['events.scan'](),
	access: () => m['events.access'](),
	claim: () => m['events.claim'](),
	claim_refused: () => m['events.claim_refused'](),
	login: () => m['events.login'](),
	signup: () => m['events.signup'](),
	revoke: () => m['events.revoke'](),
	unrevoke: () => m['events.unrevoke'](),
	expiry: () => m['events.expiry']()
};

export const eventLabel = (name: string) => EVENTS[name as EventName]?.() ?? name;

const STATUSES: Record<string, () => string> = {
	claimed: () => m['codes.status_claimed'](),
	unclaimed: () => m['codes.status_unclaimed'](),
	revoked: () => m['codes.status_revoked'](),
	expired: () => m['codes.status_expired'](),
	active: () => m['codes.status_active'](),
	signed: () => m['codes.status_signed'](),
	unsigned: () => m['codes.status_unsigned']()
};

export const statusLabel = (name: string) => STATUSES[name]?.() ?? name;

/** Labels of the shared DataTable / Pager in the UI language */
export const tableLabels = () => ({
	selectAll: m['table.select_all'](),
	selectRow: (key: string) => m['table.select_row']({ key }),
	selected: (count: number) => m['table.selected']({ count }),
	clearSelection: m['table.clear_selection'](),
	selectMatching: (count: number) => m['table.select_matching']({ count: number(count) }),
	allMatchingSelected: (count: number) => m['table.all_matching_selected']({ count: number(count) }),
	sortedAscending: m['table.sorted_ascending'](),
	sortedDescending: m['table.sorted_descending']()
});

export const pagerLabels = () => ({
	label: m['table.pages'](),
	previous: m['table.previous'](),
	next: m['table.next'](),
	range: (from: number, to: number, total: number) => m['table.range']({ from, to, total }),
	pageSize: m['table.per_page']()
});

/**
 * What people see instead of an entity key (no file names in the UI, owner 2026-09-30): `chapter:catpope:2` →
 * "Chapter 2" … The one mapping is @scutoons/ui `entityLabel`; this passes the admin's translations.
 */
export function fileLabel(key: string): string {
	return entityLabel(key, {
		chapter: (number) => m['entity.chapter']({ number }),
		season: (number) => m['entity.season']({ number }),
		toon: m['entity.toon'](),
		entry: m['entity.toon_entry'](),
		characters: m['entity.characters'](),
		moods: m['entity.moods'](),
		colorations: m['entity.colorations'](),
		requests: m['entity.requests'](),
		cast: m['entity.cast'](),
		credits: m['entity.credits'](),
		people: m['entity.people'](),
		roles: m['entity.roles'](),
		seasonScript: (number) => `${m['entity.script']()} · ${m['entity.season']({ number })}`,
		chapterScript: (number) => `${m['entity.script']()} · ${m['entity.chapter']({ number })}`,
		kanon: (subject) => `Kanon · ${subject.split('.').slice(1).join(' · ')}`
	});
}
