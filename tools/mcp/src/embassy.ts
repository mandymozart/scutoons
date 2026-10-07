// The Embassy tools (agents/EMBASSY.md): the partner network – organisations, people, leads, the communication log –
// over the existing `/api/embassy/*` with the signed-in person's session, so the API's role checks (liaison /
// ambassador / admin) decide. The agent reads, logs touchpoints, sets next dates and DRAFTS mails; it never sends:
// a draft comes back to the panel as a card, the person reviews it and sends it from the app (owner 2026-10-04: "the
// agent only drafts – a person reviews and sends", one-to-one, no mass mail).
import { randomUUID } from 'node:crypto';
import { McpServer } from '@modelcontextprotocol/server';
import * as z from 'zod';
import { ApiError, createApiClient, createEmbassyApi, createProductsApi, isStockist, ORG_TYPES, STAGES, TRACKS, type Activity, type OrgDetail } from '@scutoons/api';
import type { Config } from './config.ts';
import { apiMessage } from './server.ts';

type Result = { content: { type: 'text'; text: string }[]; isError?: boolean };
const text = (value: unknown): Result => ({ content: [{ type: 'text', text: typeof value === 'string' ? value : JSON.stringify(value, null, 1) }] });
const fail = (message: string): Result => ({ ...text(message), isError: true });
const guard =
	<A>(fn: (args: A) => Promise<Result>) =>
	async (args: A): Promise<Result> => {
		try {
			return await fn(args);
		} catch (error) {
			return fail(apiMessage(error));
		}
	};

const KINDS = ['call', 'mail_out', 'mail_in', 'dm', 'meeting', 'note'] as const;
const OUTCOMES = ['reached', 'voicemail', 'no_answer', 'call_back', 'declined'] as const;
const date = z.string().regex(/^\d{4}-\d{2}-\d{2}$/).describe('YYYY-MM-DD');
const countries = z.array(z.string().regex(/^[A-Za-z]{2}$/)).describe('ISO codes, e.g. ["DE", "AT"] – the whole list');
const upper = (codes: string[] | undefined) => (codes === undefined ? {} : { territories: codes.map((c) => c.toUpperCase()) });

/** An organisation's own fields as the agent may set them (all optional: an update changes only what it names) */
const orgFields = {
	name: z.string().min(1).max(255).optional(),
	type: z.enum(ORG_TYPES).optional(),
	city: z.string().max(255).optional(),
	country: z.string().regex(/^([A-Za-z]{2})?$/).optional().describe('ISO code, e.g. NL; "" = unknown'),
	website: z.string().max(1024).optional(),
	email: z.string().max(255).optional().describe('the organisation\'s general address; "" clears it'),
	phone: z.string().max(64).optional(),
	links: z
		.array(z.string())
		.optional()
		.describe('social profiles and other pages (Instagram, Facebook, Bandcamp, Soundcloud …) as full URLs – the whole list: read_org first and keep the ones that stay'),
	tags: z.array(z.string()).optional().describe('genre / scene tags – the whole list'),
	territories: countries.optional().describe('countries they cover (agents, distributors)'),
	tier: z.enum(['A', 'B', 'C', '']).optional().describe('"" = none'),
	source: z.string().max(255).optional().describe('where the contact came from, e.g. "tip from a friend"'),
	notes: z.string().max(20000).optional().describe('replaces the notes – read them first and keep what stays'),
	do_not_contact: z.boolean().optional().describe('true = they asked not to be contacted: no mail, no DM, no call'),
	// stores (type store / record_shop): the public "Where to buy" page
	listed: z.boolean().optional().describe('stores: shown on scutoons.com'),
	independent: z.boolean().optional().describe('stores: a small / independent vendor (listed first)'),
	address: z.string().max(255).optional().describe('stores: street + postcode (public)'),
	opening: z.string().max(255).optional().describe('stores: opening info as the store writes it (public)'),
	carries: z.array(z.string()).optional().describe('stores: product ids (list_releases) it carries – the whole list')
};

/** A person's fields (all optional: an update changes only what it names) */
const personFields = {
	name: z.string().min(1).max(255).optional(),
	role: z.string().max(255).optional().describe('booker, editor, buyer, owner …'),
	email: z.string().max(255).optional(),
	phone: z.string().max(64).optional(),
	link: z.string().max(1024).optional().describe('their social profile or page (Instagram …) as a full URL – a DM goes there'),
	language: z.string().max(16).optional().describe('e.g. de, en, nl'),
	territories: countries.optional(),
	notes: z.string().max(20000).optional(),
	do_not_contact: z.boolean().optional().describe('true = they asked not to be contacted')
};

/** a log entry as the agent reads it (long mail texts cut) */
const entry = (a: Activity) => ({
	id: a.id,
	at: a.at,
	kind: a.kind,
	track: a.track,
	person_id: a.person_id,
	outcome: a.outcome,
	by: a.by_email,
	...(a.subject ? { subject: a.subject } : {}),
	...(a.mail_to ? { to: a.mail_to } : {}),
	...(a.mail_from ? { from: a.mail_from } : {}),
	note: a.note.length > 1500 ? `${a.note.slice(0, 1500)} …` : a.note
});

/** Who may be written to at an organisation: its address and its people's, minus everyone who opted out */
function recipients(detail: OrgDetail) {
	const out: { email: string; person_id: string | null; name: string; language: string }[] = [];
	if (detail.org.email && !detail.org.do_not_contact) out.push({ email: detail.org.email, person_id: null, name: detail.org.name, language: '' });
	if (!detail.org.do_not_contact)
		for (const p of detail.people) if (p.email && !p.do_not_contact) out.push({ email: p.email, person_id: p.id, name: p.name, language: p.language });
	return out;
}

export function createEmbassyServer(config: Config): McpServer {
	const client = createApiClient({ baseUrl: config.api, getToken: () => config.token });
	const embassy = createEmbassyApi(client);
	const catalog = createProductsApi(client);

	const server = new McpServer(
		{ name: 'scutoons-embassy', version: '0.1.0' },
		{ instructions: 'SCUTOONS Embassy: the partner network (stages, vinyl outlets, bookers, press). You have the rights of the signed-in liaison / ambassador. You draft mails; people send them.' }
	);
	const read = { readOnlyHint: true, openWorldHint: false };
	const write = { readOnlyHint: false, destructiveHint: false, openWorldHint: false };

	server.registerTool(
		'today',
		{
			description: 'The due list: leads whose next check date is today or earlier (who to call or write), with the organisation, its people and the last log entry. owner: "me" (default), "all", "none" (unowned) or a user id.',
			inputSchema: z.object({ owner: z.string().optional() }),
			annotations: read
		},
		guard(async ({ owner }) => {
			const r = await embassy.today(owner ?? 'me');
			return text({
				today: r.today,
				unscheduled: r.unscheduled,
				due: r.due.map((d) => ({
					org_id: d.org.id,
					org: d.org.name,
					city: d.org.city,
					track: d.lead.track,
					stage: d.lead.stage,
					next_at: d.lead.next_at,
					overdue_days: d.overdue_days,
					owner: d.lead.owner_email,
					last: d.last_activity ? entry(d.last_activity) : null
				}))
			});
		})
	);

	server.registerTool(
		'my_portion',
		{
			description:
				"The signed-in person's portion this week at their own pace (contacts per week): how many they did, how many are left, today's slice first (today_count), then the rest of the week; each item with its group (due call, answered before / quiet now, first contact), a hint (why) and the last log entry. What doesn't fit waits for next week – never push them past their pace.",
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			const p = await embassy.portion('me');
			return text({
				today: p.today,
				week: p.week,
				today_count: p.today_count,
				waiting_next_week: p.more,
				items: p.items.map((i) => ({
					group: i.group,
					hint: i.hint,
					org_id: i.org.id,
					org: i.org.name,
					city: i.org.city,
					// the organisation's own address / number (a mail can go there when no person is known)
					email: i.org.email,
					// social profiles (Instagram, Facebook, Bandcamp …): a DM is a valid first contact when no address is known
					links: i.org.links,
					phone: i.org.phone,
					track: i.lead.track,
					stage: i.lead.stage,
					people: i.people.map((x) => ({ id: x.id, name: x.name, role: x.role, phone: x.phone, email: x.email, link: x.link })),
					last: i.last_activity ? entry(i.last_activity) : null
				}))
			});
		})
	);

	server.registerTool(
		'search_orgs',
		{
			description:
				'Find organisations (venues, festivals, shops, distributors, agents, radios, press …) by text (name, city, tags, people) and filters. Answers id, name, type, place, tier, whether they opted out, and their leads (track, stage, next date, owner).',
			inputSchema: z.object({
				q: z.string().optional().describe('free text'),
				track: z.enum(TRACKS).optional(),
				stage: z.enum(STAGES).optional(),
				type: z.string().optional(),
				country: z.string().optional().describe('ISO code, e.g. AT'),
				tier: z.enum(['A', 'B', 'C']).optional(),
				owner: z.string().optional().describe('"me", "none" or a user id'),
				due: z.boolean().optional().describe('only leads due today or earlier'),
				quiet: z.number().int().min(1).max(3650).optional().describe('only leads that answered before (a reply, a meeting, a reached call) and then had nothing for this many days (90 is a good default), no date ahead, not "no", not opted out – the people worth a call again'),
				limit: z.number().int().min(1).max(100).optional()
			}),
			annotations: read
		},
		guard(async ({ due, quiet, limit, ...filters }) => {
			const r = await embassy.orgs({ ...filters, ...(due ? { due: 1 } : {}), ...(quiet ? { quiet } : {}), limit: limit ?? 25 });
			return text({
				total: r.total,
				items: r.items.map((o) => ({
					id: o.id,
					name: o.name,
					type: o.type,
					city: o.city,
					country: o.country,
					tier: o.tier,
					do_not_contact: o.do_not_contact,
					people: o.people_count,
					leads: o.leads.map((l) => ({ track: l.track, stage: l.stage, next_at: l.next_at, owner: l.owner_email }))
				}))
			});
		})
	);

	server.registerTool(
		'read_org',
		{
			description:
				'One organisation with everything known: details (incl. `links` = social profiles), notes, people (role, email, phone, `link` = their profile, language, opted out), leads per track and the communication log (newest first; mails with subject and text). `recipients` = the addresses you may draft to (opted-out ones are left out; language "" = unknown, go by the country).',
			inputSchema: z.object({ id: z.string(), log: z.number().int().min(0).max(200).optional().describe('log entries, default 30') }),
			annotations: read
		},
		guard(async ({ id, log }) => {
			const d = await embassy.org(id);
			const { org } = d;
			return text({
				org: {
					id: org.id,
					name: org.name,
					type: org.type,
					city: org.city,
					country: org.country,
					website: org.website,
					links: org.links,
					email: org.email,
					phone: org.phone,
					tags: org.tags,
					tier: org.tier,
					territories: org.territories,
					source: org.source,
					notes: org.notes,
					do_not_contact: org.do_not_contact,
					...(isStockist(org.type) ? { listed: org.listed, independent: org.independent, address: org.address, opening: org.opening, carries: org.carries } : {})
				},
				people: d.people.map((p) => ({ id: p.id, name: p.name, role: p.role, email: p.email, phone: p.phone, link: p.link, language: p.language, territories: p.territories, notes: p.notes, do_not_contact: p.do_not_contact })),
				leads: d.leads.map((l) => ({ track: l.track, stage: l.stage, next_at: l.next_at, owner: l.owner_email, last_contact_at: l.last_contact_at })),
				recipients: recipients(d),
				log: d.activities.slice(0, log ?? 30).map(entry)
			});
		})
	);

	server.registerTool(
		'list_team',
		{ description: 'The people working in the Embassy (id, email, access) – for assigning a lead.', inputSchema: z.object({}), annotations: read },
		guard(async () => text((await embassy.team()).items))
	);

	server.registerTool(
		'list_releases',
		{
			description:
				'What there is to pitch: the music albums (title per language, tracks) and the shop products (name, category, link, price) of the SCUTOONS catalog. Use the links as they are; never invent a release, date or link.',
			inputSchema: z.object({}),
			annotations: read
		},
		guard(async () => {
			const [items, products] = await Promise.all([client.get<{ albums?: any[] }>('/items/', { auth: false }), catalog.fetchProducts()]);
			return text({
				albums: (items.albums ?? []).map((a) => ({ id: a.id, toon: a.toon, title: a.title, tracks: (a.tracks ?? []).map((t: any) => t.title) })),
				products: products.products.map((p) => ({ id: p.id, toon: p.toon, name: p.name, category: p.category, link: p.link, price: `${p.price} ${products.currency}` }))
			});
		})
	);

	server.registerTool(
		'log_activity',
		{
			description:
				'Writes a touchpoint into an organisation\'s communication log and moves the lead on like the app does: call reached / mail_in / meeting → in conversation, next check in 10 days; voicemail / no_answer → 3 days; call_back needs next_at; declined → stage "no"; mail_out → contacted, call in 6 days; dm (a message the person sent on social media / a messenger) → contacted, check in 6 days. Mails sent from the app are logged on their own – use mail_out only for a mail sent elsewhere. Only log what the person told you happened.',
			inputSchema: z.object({
				org_id: z.string(),
				kind: z.enum(KINDS),
				track: z.enum(TRACKS).optional().describe('the lead this is about'),
				person_id: z.string().optional(),
				outcome: z.enum(OUTCOMES).optional().describe('calls only (required for a call)'),
				note: z.string().optional(),
				next_at: date.optional().describe('overrides the automatic next check date'),
				stage: z.enum(STAGES).optional().describe('overrides the automatic stage')
			}),
			annotations: write
		},
		guard(async ({ org_id, ...input }) => {
			const r = await embassy.log(org_id, input);
			return text({ logged: r.activity.id, lead: r.lead ? { track: r.lead.track, stage: r.lead.stage, next_at: r.lead.next_at } : null });
		})
	);

	server.registerTool(
		'set_lead',
		{
			description: 'Sets a lead\'s stage, next check date or owner (one lead per organisation × track; creates it when missing).',
			inputSchema: z.object({
				org_id: z.string(),
				track: z.enum(TRACKS),
				stage: z.enum(STAGES).optional(),
				next_at: date.nullable().optional(),
				owner_id: z.string().nullable().optional().describe('a user id from list_team, null = nobody')
			}),
			annotations: write
		},
		guard(async ({ org_id, track, ...input }) => {
			const { lead } = await embassy.setLead(org_id, track, input);
			return text({ track: lead.track, stage: lead.stage, next_at: lead.next_at, owner: lead.owner_email });
		})
	);

	server.registerTool(
		'create_org',
		{
			description:
				'Adds a new organisation (and optionally its people and its leads, one per track – stage, next date, owner) like the app\'s "New organisation". search_orgs first: the same name in the same city / country answers "duplicate" with the existing id – use that one instead. Only fill in what the person told you or what you checked (website, social links, city, country); never invent contact data. Answers the new id.',
			inputSchema: z.object({
				...orgFields,
				name: z.string().min(1).max(255),
				type: z.enum(ORG_TYPES),
				people: z
					.array(z.object({ ...personFields, name: z.string().min(1).max(255) }))
					.optional()
					.describe('the people known there'),
				leads: z
					.array(
						z.object({
							track: z.enum(TRACKS),
							stage: z.enum(STAGES).optional().describe('default research'),
							next_at: date.nullable().optional(),
							owner_id: z.string().nullable().optional().describe('a user id from list_team')
						})
					)
					.optional()
			}),
			annotations: write
		},
		guard(async ({ country, territories, people, ...input }) => {
			try {
				const { org, people: added, leads } = await embassy.createOrg({
					...input,
					...(country !== undefined ? { country: country.toUpperCase() } : {}),
					...upper(territories),
					...(people ? { people: people.map(({ territories: t, ...p }) => ({ ...p, ...upper(t) })) } : {})
				});
				return text({ id: org.id, name: org.name, people: added.map((p) => ({ id: p.id, name: p.name })), leads: leads.map((l) => ({ track: l.track, stage: l.stage, next_at: l.next_at, owner: l.owner_email })) });
			} catch (error) {
				if (error instanceof ApiError && error.code === 'duplicate')
					return fail(`duplicate: ${input.name} exists already (id ${String((error.body as Record<string, unknown> | undefined)?.id ?? '?')}) – use that organisation.`);
				throw error;
			}
		})
	);

	server.registerTool(
		'update_org',
		{
			description:
				"Changes an organisation's details – only the fields you name: contact details, social links, type, place, tags, tier, notes, source, do not contact, the store fields. Lists (links, tags, territories, carries) and notes are replaced as a whole: read_org first and keep what stays. Only set what the person told you or what you checked; never invent contact data.",
			inputSchema: z.object({ org_id: z.string(), ...orgFields }),
			annotations: write
		},
		guard(async ({ org_id, country, territories, ...input }) => {
			const { org } = await embassy.updateOrg(org_id, { ...input, ...(country !== undefined ? { country: country.toUpperCase() } : {}), ...upper(territories) });
			return text({ id: org.id, name: org.name, changed: [...Object.keys(input), ...(country !== undefined ? ['country'] : []), ...(territories ? ['territories'] : [])] });
		})
	);

	server.registerTool(
		'add_person',
		{
			description: 'Adds a person (booker, editor, buyer …) to an organisation: name, role, email, phone, social profile link, language, notes. Only what the person told you or what you checked.',
			inputSchema: z.object({ org_id: z.string(), ...personFields, name: z.string().min(1).max(255) }),
			annotations: write
		},
		guard(async ({ org_id, territories, ...input }) => {
			const { person } = await embassy.addPerson(org_id, { ...input, ...upper(territories) });
			return text({ id: person.id, name: person.name, org_id });
		})
	);

	server.registerTool(
		'update_person',
		{
			description: "Changes a person's details – only the fields you name: role, email, phone, social profile link, language, notes, do not contact. Notes are replaced as a whole: read_org first.",
			inputSchema: z.object({ person_id: z.string(), ...personFields }),
			annotations: write
		},
		guard(async ({ person_id, territories, ...input }) => {
			const { person } = await embassy.updatePerson(person_id, { ...input, ...upper(territories) });
			return text({ id: person.id, name: person.name, org_id: person.org_id, changed: [...Object.keys(input), ...(territories ? ['territories'] : [])] });
		})
	);

	server.registerTool(
		'delete_person',
		{
			description:
				'Removes a person from an organisation (ambassadors only; their log entries stay, without the person). Only when the person you work for asks for exactly this – someone who left, or a duplicate. Someone who does not want to hear from us is not deleted: update_person do_not_contact.',
			inputSchema: z.object({ person_id: z.string() }),
			annotations: { readOnlyHint: false, destructiveHint: true, openWorldHint: false }
		},
		guard(async ({ person_id }) => {
			await embassy.deletePerson(person_id);
			return text({ deleted: person_id });
		})
	);

	server.registerTool(
		'draft_mail',
		{
			description:
				'Puts a mail draft in front of the person as a card: they edit it, then send it from the app (one-to-one, from the shared mailbox) – you never send. `to` must be one of read_org\'s `recipients` (people who opted out never get mail). Write in the recipient\'s language. The opt-out line is added when sent – don\'t write one. Several recipients = several drafts, each individual.',
			inputSchema: z.object({
				org_id: z.string(),
				to: z.string().describe('an address from read_org recipients'),
				track: z.enum(TRACKS).optional(),
				language: z.string().describe('the language the mail is written in, e.g. de, en'),
				subject: z.string().min(1).max(300),
				text: z.string().min(1).max(20000).describe('plain text, signed by the person (see the system prompt)'),
				reply_to_activity_id: z.string().optional().describe('a mail log entry this answers (keeps the thread)')
			}),
			annotations: read
		},
		guard(async ({ org_id, to, track, language, subject, text: body, reply_to_activity_id }) => {
			const detail = await embassy.org(org_id);
			const recipient = recipients(detail).find((r) => r.email.toLowerCase() === to.trim().toLowerCase());
			if (!recipient) {
				const opted = detail.org.do_not_contact || detail.people.some((p) => p.do_not_contact && p.email.toLowerCase() === to.trim().toLowerCase());
				return fail(opted ? `${to} opted out (do not contact) – no mail to them, ever.` : `${to} is not an address of ${detail.org.name} – use one of read_org's recipients.`);
			}
			// `id`: the panel remembers which drafts were sent, so a reopened chat doesn't offer to send one twice
			const draft = {
				id: randomUUID(),
				kind: 'mail_draft',
				org_id,
				org: detail.org.name,
				to: recipient.email,
				name: recipient.name,
				person_id: recipient.person_id,
				track: track ?? null,
				language,
				subject,
				text: body,
				reply_to_activity_id: reply_to_activity_id ?? null
			};
			return text({ draft, shown: 'The person sees the draft as a card and sends it themselves. Do not repeat the mail text in your answer.' });
		})
	);

	server.registerTool(
		'check_inbox',
		{
			description: 'Reads new replies from the shared mailbox into the log (replies are matched to organisations by thread or sender; a reply stops the follow-ups). Answers how many were new.',
			inputSchema: z.object({}),
			annotations: write
		},
		guard(async () => {
			const r = await embassy.syncInbox();
			return text(r.configured ? r : 'The inbox is not set up on this server (EMBASSY_IMAP_*).');
		})
	);

	return server;
}
