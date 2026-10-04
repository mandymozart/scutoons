// Embassy API (server/docs/API.md "Embassy"): `/embassy/*`, session + role liaison / ambassador (admin counts as
// ambassador). The partner network: organisations, people, one lead per organisation × track, activities.
import type { ApiClient } from './client.ts';
import { toQuery, type ListParams } from './admin.ts';

export type EmbassyAccess = 'liaison' | 'ambassador';
export type Track = 'live' | 'vinyl' | 'booking' | 'promo';
export type Stage = 'research' | 'ready' | 'contacted' | 'in_conversation' | 'won' | 'not_now' | 'no' | 'dormant';
export type ActivityKind = 'mail_out' | 'mail_in' | 'call' | 'dm' | 'meeting' | 'note';
export type CallOutcome = 'reached' | 'voicemail' | 'no_answer' | 'call_back' | 'declined';
export type Tier = 'A' | 'B' | 'C';

export const TRACKS: Track[] = ['live', 'vinyl', 'booking', 'promo'];
export const STAGES: Stage[] = ['research', 'ready', 'contacted', 'in_conversation', 'won', 'not_now', 'no', 'dormant'];
export const ORG_TYPES = [
	'venue', 'festival', 'theatre', 'art_space', 'residency', 'radio', 'promoter', 'agency', 'distributor',
	'store', 'record_shop', 'label', 'blog', 'magazine', 'podcast', 'journalist', 'playlist', 'newsletter', 'ad_channel', 'other'
] as const;
export type OrgType = (typeof ORG_TYPES)[number];
/** Types that sell to readers: they carry products and can be listed on scutoons.com/stores (agents/EMBASSY.md "Stores") */
export const STOCKIST_TYPES: OrgType[] = ['store', 'record_shop'];
export const isStockist = (type: string | null | undefined): boolean => STOCKIST_TYPES.includes(type as OrgType);
export const ACTIVITY_KINDS: ActivityKind[] = ['mail_out', 'mail_in', 'call', 'dm', 'meeting', 'note'];
export const CALL_OUTCOMES: CallOutcome[] = ['reached', 'voicemail', 'no_answer', 'call_back', 'declined'];
export const TIERS: Tier[] = ['A', 'B', 'C'];

export interface Org {
	id: string;
	name: string;
	type: OrgType;
	city: string;
	/** ISO 3166-1 alpha-2, upper case, '' = unknown */
	country: string;
	website: string;
	email: string;
	phone: string;
	links: string[];
	tags: string[];
	/** countries they cover (agents) */
	territories: string[];
	tier: Tier | null;
	source: string;
	notes: string;
	do_not_contact: boolean;
	created_at: string;
	updated_at: string;
	/** stores (STOCKIST_TYPES): shown on the public "Where to buy" page (with at least one product) */
	listed: boolean;
	/** stores: a small / independent vendor – listed first */
	independent: boolean;
	/** stores: street + postcode (public) */
	address: string;
	/** stores: opening info as the store writes it (public) */
	opening: string;
	/** stores: product ids (`/products/`) it carries */
	carries: string[];
}

export interface Person {
	id: string;
	org_id: string;
	name: string;
	role: string;
	email: string;
	phone: string;
	link: string;
	language: string;
	territories: string[];
	notes: string;
	do_not_contact: boolean;
}

export interface Lead {
	id: string;
	org_id: string;
	track: Track;
	stage: Stage;
	owner_id: string | null;
	owner_email: string | null;
	/** YYYY-MM-DD, the next check date */
	next_at: string | null;
	last_contact_at: string | null;
	updated_at: string;
}

export interface Activity {
	id: string;
	org_id: string;
	lead_id: string | null;
	track: Track | null;
	person_id: string | null;
	kind: ActivityKind;
	outcome: CallOutcome | null;
	note: string;
	by_email: string;
	/** UTC `YYYY-MM-DD HH:MM:SS` */
	at: string;
	/** mails (sent from the app / read back from the inbox): subject, addresses, the Message-ID of the thread */
	subject?: string | null;
	mail_from?: string | null;
	mail_to?: string | null;
	message_id?: string | null;
}

export interface OrgSummary extends Org {
	people_count: number;
	leads: Lead[];
}

export interface OrgDetail {
	org: Org;
	people: Person[];
	leads: Lead[];
	activities: Activity[];
}

export interface DueItem {
	lead: Lead;
	org: Org;
	people: Person[];
	last_activity: Activity | null;
	overdue_days: number;
}

export interface TeamMember {
	id: string;
	email: string;
	access: EmbassyAccess;
}

export interface ImportReport {
	dry_run: boolean;
	created: number;
	merged: number;
	people_added: number;
	leads_added: number;
	errors: { index: number; name: string; error: string }[];
}

export type OrgInput = Partial<Omit<Org, 'id' | 'created_at' | 'updated_at' | 'tier'>> & {
	tier?: Tier | null | '';
	people?: Partial<Person>[];
	leads?: { track: Track; stage?: Stage; next_at?: string | null; owner_id?: string | null }[];
	force?: boolean;
};

export interface LogInput {
	kind: ActivityKind;
	track?: Track | null;
	person_id?: string | null;
	outcome?: CallOutcome | null;
	note?: string;
	stage?: Stage;
	next_at?: string | null;
}

/**
 * A team member's own pace (owner 2026-10-05: liaisons work at their own pace): contacts per week and the reminder
 * mail; `null` in `/embassy/me` = not onboarded yet. Every 4 weeks `checkin_due`: "is your pace right?"
 */
export interface EmbassyPace {
	per_week: number;
	reminder: PaceReminder;
	onboarded_at: string;
	/** the last check-in (or the onboarding) */
	asked_at: string;
	checkin_due: boolean;
}
export type PaceReminder = 'weekly' | 'daily' | 'off';
export const PACE_REMINDERS: PaceReminder[] = ['weekly', 'daily', 'off'];
export type CheckinAnswer = 'less' | 'fine' | 'more';

/** Why an item is in the portion – the app writes it in its language */
export interface PortionHint {
	kind: string;
	days?: number;
	subject?: string;
	date?: string;
}

export interface PortionItem extends DueItem {
	group: 'due' | 'quiet' | 'first';
	hint: PortionHint;
}

/** This week's portion of a team member: what fits their pace, in order due → quiet → first contacts */
export interface Portion {
	today: string;
	week: { start: string; end: string; pace: number; done: number; left: number };
	/** how many of `items` are today's slice (what was left this morning, spread over the working days left, minus today's) */
	today_count: number;
	/** contacts done today */
	today_done: number;
	items: PortionItem[];
	/** what didn't fit this week */
	more: { due: number; quiet: number; first: number };
	/** leads they own */
	owned: number;
}

export interface TeamPaceRow {
	user: TeamMember;
	pace: EmbassyPace | null;
	owned: number;
	/** the last 4 weeks, this one last */
	weeks: { start: string; done: number }[];
	last_checkin: { at: string; answer: CheckinAnswer; per_week_before: number; per_week_after: number } | null;
}

/** A mail a person sends from the app (`POST /embassy/orgs/{id}/mail`): one-to-one, the server adds the opt-out line */
export interface MailInput {
	/** the organisation's address or one of its people's */
	to: string;
	person_id?: string | null;
	track?: Track | null;
	subject: string;
	text: string;
	/** a mail activity of this organisation to answer (threading) */
	reply_to_activity_id?: string | null;
}

/** The shared mailbox's inbox sync (IMAP): replies land in the log */
export interface InboxState {
	configured: boolean;
	synced_at?: string | null;
	error?: string | null;
	matched?: number;
	skipped?: number;
	new_activities?: number;
}

const seg = encodeURIComponent;

export type EmbassyApi = ReturnType<typeof createEmbassyApi>;

export function createEmbassyApi(api: ApiClient) {
	return {
		me: () => api.get<{ access: EmbassyAccess; user: { id: string; email: string }; pace?: EmbassyPace | null }>('/embassy/me'),
		setPace: (input: { per_week: number; reminder: PaceReminder }) => api.put<{ pace: EmbassyPace }>('/embassy/pace', input),
		checkin: (answer: CheckinAnswer) => api.post<{ pace: EmbassyPace }>('/embassy/pace/checkin', { answer }),
		portion: (owner: string = 'me') => api.get<Portion>(`/embassy/portion${toQuery({ owner })}`),
		teamPace: () => api.get<{ items: TeamPaceRow[] }>('/embassy/team/pace'),
		team: () => api.get<{ items: TeamMember[] }>('/embassy/team'),
		today: (owner: string = 'me') => api.get<{ today: string; due: DueItem[]; unscheduled: number }>(`/embassy/today${toQuery({ owner })}`),
		orgs: (params: ListParams = {}) => api.get<{ items: OrgSummary[]; total: number }>(`/embassy/orgs${toQuery(params)}`),
		org: (id: string) => api.get<OrgDetail>(`/embassy/orgs/${seg(id)}`),
		createOrg: (input: OrgInput) => api.post<OrgDetail>('/embassy/orgs', input),
		updateOrg: (id: string, input: OrgInput) => api.put<OrgDetail>(`/embassy/orgs/${seg(id)}`, input),
		deleteOrg: (id: string) => api.delete(`/embassy/orgs/${seg(id)}`),
		addPerson: (orgId: string, person: Partial<Person>) => api.post<{ person: Person }>(`/embassy/orgs/${seg(orgId)}/people`, person),
		updatePerson: (id: string, person: Partial<Person>) => api.put<{ person: Person }>(`/embassy/people/${seg(id)}`, person),
		deletePerson: (id: string) => api.delete(`/embassy/people/${seg(id)}`),
		setLead: (orgId: string, track: Track, input: { stage?: Stage; owner_id?: string | null; next_at?: string | null }) =>
			api.put<{ lead: Lead }>(`/embassy/orgs/${seg(orgId)}/leads/${seg(track)}`, input),
		log: (orgId: string, input: LogInput) => api.post<{ activity: Activity; lead: Lead | null }>(`/embassy/orgs/${seg(orgId)}/activities`, input),
		sendMail: (orgId: string, input: MailInput) => api.post<{ activity: Activity; lead: Lead | null }>(`/embassy/orgs/${seg(orgId)}/mail`, input),
		inbox: () => api.get<InboxState>('/embassy/inbox'),
		syncInbox: (force = false) => api.post<InboxState>('/embassy/inbox/sync', force ? { force } : {}),
		import: (organisations: OrgInput[], dryRun: boolean) =>
			api.post<ImportReport>('/embassy/import', { organisations, dry_run: dryRun })
	};
}
