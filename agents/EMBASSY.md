# PLAN — Embassy (embassy.scutoons.com), the partner network of Top Toy / SCUTOONS

Written 2026-10-01. Branch `embassy` (worktree `.claude/worktrees/embassy`), kept out of the 2026-10-04 release
(`platform` → `main`); merge after the release. Status: **phase 0 + phase 1 built** (see "Built" below).

## Goal

One place where the owner and part-time helpers find, contact and keep in touch with the people who make the music
travel: stages, vinyl outlets, bookers and press. Every contact has a known state and a next check date, so the lists
get worked through again and again instead of going stale.

## Name

The job isn't really "sales rep". In music it's split into **booker** (stages), **plugger / promoter** (radio, press)
and **label manager** (distribution). The umbrella term the scene uses is **artist relations / partner relations**.

**Decided (owner, 2026-10-01):** the app is **Embassy** at `embassy.scutoons.com`, on the same API
(`api.scutoons.com`, `/api/embassy/*`). People: part-time helpers are **Liaisons** (role `liaison`), someone hired full
time becomes an **Ambassador** (role `ambassador`). Rejected: Network, Circle, Relations, Portal.

## The four tracks (= pipelines)

| Track | Who | We want | "Won" means |
|---|---|---|---|
| **Live** | venues, festivals, theatres, art/performance spaces, residencies, web radios (live sessions) | a slot | gig / show / session booked |
| **Vinyl** | distributors, record shops, mail-order | records in their shop or catalogue | first order / consignment |
| **Booking** | agents (per region or general) | representation | agent signed for region X |
| **Promo** | radios, blogs, magazines, podcasts, journalists, playlists; ad channels (social, posters) | coverage, airplay | review / feature / airplay / post |

One organisation can be in several tracks (a festival with a radio, a shop that also books shows).

**Fifth audience: members** (scutoons accounts, not leads). They get **updates** about new items, chapters, releases and
gigs. Unlike partners they already said yes: only members with `users.marketing_opt_in` (exists, set at sign-up /
account page) get mails, with an unsubscribe link that clears the flag. These are broadcasts (one pitch → many
members), not one-to-one pipelines, so they sit next to the tracks, not in them.

**Region: Europe first.** Bookers and agents often cover specific territories (e.g. one agent for DACH, another for
Benelux, Scandinavia or the UK). So booking is planned **per territory**, not per agency.

## Data model (small, per "keep the configuration small")

- **Organisation**: name, type, city, country, website, socials, language, genre tags, tier (A/B/C), source
  (Top List, own research, referral …).
- **Person**: organisation, name, role (booker, editor, buyer …), email / phone / Instagram, language, notes,
  **do-not-contact** flag + date (opt-out).
- **Lead** (organisation × track): stage, owner (who's in charge), **next check date**, last contact date.
  Stages: `research` → `ready` → `contacted` → `in conversation` → `won` / `not now` / `no` → `dormant`.
- **Activity**: email out/in, DM, call, meeting, note; date, by whom, linked thread.
- **Pitch**: what we send (new release, EPK, video, tour dates) with links. A new pitch is what reopens dormant leads.
- **Template**: per track × language × step (first contact, follow-up, update, thank-you), plus a **call note** per step
  (what to say on the phone, which mail to refer to).
- **Territories**: a list of countries on an organisation / person (for agents: the countries they book). The Booking
  track shows a Europe coverage view: which countries have an agent, which are being talked to, which are still open.
- **Member update**: a pitch sent to opted-in members (subject, text de/en, links to the item/chapter), with its send
  date and number of recipients. Reuses the pitch, so one new item can be announced to members and pitched to
  partners in one go.
- **Research run**: one AI research job: the brief, who asked, when, the candidates it found (see below).

Nothing else until a screen edits it or code reads it.

## Research strategy

The Top List today (`C:\Development\toplist\The Top List.csv`): 167 entries, but **only 18 have an email address**.
69 radios, 38 journalists (all Vital Weekly writers with no contact), 32 venues, 12 festivals, 9 blogs/magazines,
1 agency. **No distributors, record shops or ad channels at all.** So the list is mostly a starting index, and
filling it in is the main work.

1. **Import + clean**: Top List CSV/XLSX → organisations + persons; dedupe (Cashmere, Noods, LYL, Raheem are in twice);
   split the multi-contact cells (Powerline Agency → 12 people); mark `closed` entries (redlight radio).
2. **Enrich** (AI-assisted, human checks): per organisation find the official submission / booking / demo address or
   form, contact person, language, genre fit, last activity (still running?). Store the source URL of every fact.
3. **Fill the gaps**, one sweep per track:
   - Vinyl: distributors with an experimental/electronic catalogue (e.g. Kompakt, Rough Trade, Cargo, Clone, Hard Wax,
     Staalplaat → find the full list), record shops per city we play in.
   - Booking: agencies whose roster looks like ours (start from artists we're compared to: who books them?).
   - Promo: playlists, newsletters, ad channels (posters per city, Instagram/Meta ads, magazine ads).
4. **Snowball**: every reply or gig produces new names ("you should talk to …"), captured as source `referral`.
5. **Tier**: A = perfect fit and reachable; B = good fit; C = long shot. Tier sets the cadence.
6. **Rhythm**: research isn't a phase, it's a weekly task: N new organisations per track per week (owner sets N).
7. **AI research runs, on demand.** The owner tells an agent in a sentence what to look for, e.g. "venues like
   Dachstock and Institut für die Zukunft in Poland and Czechia", "record shops in cities where we played", "agents
   who book artists like X in Scandinavia". The agent uses what we've learned so far:
   - **fit profile**: types, sizes, genres and cities of the leads that replied or were won, plus the owner's notes on
     why they fit (the AI writes it, the owner edits it);
   - **lookalikes and links**: same scene, same artists on the line-up, partner venues, the festival's radio, the
     radio's shows, who reviews whom;
   - **referrals** and dead ends (no / closed / wrong fit), so it doesn't suggest them again.
   It returns **candidates**, each with a reason ("booked A and B, same size as Dachstock"), sources and the contact
   path it found. Candidates are deduped against the database and land in the research queue. A person accepts or
   rejects them, and the reasons for rejecting feed back into the fit profile. **A research run never contacts
   anyone.** Runs can also be scheduled (e.g. monthly "new festivals announced for next summer in Europe").

## Communication strategy

Relatability beats volume: few, specific, personal messages.

- **Every first contact is individual**: one line that proves we know them (a show they ran, an article, a shared
  artist), what we are (SCUTOONS, one sentence), one concrete ask, one link (EPK / release). Short. In their language
  (de/en first, others via AI translation reviewed).
- **Mail first, then a phone call** (owner decision 2026-10-01: follow-ups old school, by phone). The mail is the
  reference ("we sent you our EPK on Tuesday …"); the call is where the relationship starts. Enrichment therefore also
  looks for **phone numbers** and office hours.
- **Cadence** (defaults, editable per lead; the follow-up steps are automated as **call tasks** on the due list, with
  the call note and the mail it refers to on one screen and an automatic mail sent beforehand if the step has one):
  - cold: first mail → call after 5–7 days → second call / short mail after 21 days → `not now`, wake up on the next pitch.
  - in conversation: follow up whenever it goes quiet for 10 days.
  - won / partners: touch every 60 (A) / 90 (B) / 180 (C) days, or when there's news: thank-yous, photos from the gig,
    new release, tour in their city.
- **Reply = sequence stops.** Nobody gets a follow-up after they answered.
- **Opt-out respected forever** (do-not-contact flag, checked before every send).
- **News drives updates**: a new pitch (release, video, chapter) creates a "who should hear about this?" list across
  all tracks, filtered by tier and fit.
- **One voice**: helpers write under their own first name, from a shared mailbox, with the same signature and EPK.
  No "Top Toy" in mails or DMs (owner 2026-10-07: dropped; was "Top Toy / [their name]"), and the agent assumes
  nothing about who the person is or where they work.

## Automation — what machines do, what people do

| Step | Automated | Human |
|---|---|---|
| Import, dedupe, enrich | AI agent researches websites, proposes addresses + facts with sources | approves / corrects |
| Due list | daily "today: 8 calls, 4 follow-up mails, 5 new" per person, push/email reminder | works the list |
| Calls | call task with number (tap to call), call note, linked mail, quick outcome buttons (reached / voicemail / call back on … / no) → next date set automatically | makes the call |
| Research | AI research runs (on demand or scheduled) → candidates with reasons + sources | accepts / rejects |
| Member updates | new item/chapter in the catalog or creator → draft update for opted-in members | edits, sends |
| Draft | AI drafts from lead facts + thread history + template + current pitch | edits, **sends** (1 click) |
| Send | from the real mailbox (SMTP), one-to-one, threaded | – |
| Inbox | IMAP sync: replies matched to leads, sequence stopped, AI suggests the stage + next date | confirms |
| Reporting | per track: contacted, replies, won, per helper | – |

First mails: **a person approves every send**. ~~Follow-ups: automated (owner OK 2026-10-01), mostly as call tasks; an
automatic follow-up mail only in a running thread, and never after a reply or opt-out.~~ **No automatic follow-ups
(owner 2026-10-04):** nothing is sent on its own; the tool helps compose mails and prepare phone follow-ups (the next
date puts a call on the Today list), and keeps track of dormant contacts that were responsive in the past.

**Legal (check before sending, not legal advice):** in Austria (TKG 2021 §174) and Germany (UWG §7) unsolicited
advertising emails generally need prior consent, also B2B. Individual, relevant inquiries to published
booking/submission addresses are common practice, bulk sequences are the risk. So: one-to-one mails, no mass blasts,
opt-out in every mail, keep the source of each address, delete on request (GDPR). Get a short opinion from someone who
knows AT/DE law before automated follow-ups. Phone calls have their own rules per country (Germany allows B2B calls
with presumed interest, Austria is stricter), so check this per country too. Member updates are fine: they're
opt-in.

## On the platform

Fits the existing structure (PLATFORM.md): new SvelteKit static app + API module + role.

- `embassy/` (SvelteKit, `@scutoons/ui`, `@scutoons/api`) → `embassy.scutoons.com`, mobile first (the owner works on
  the go): **Today** (due list), **Leads** (per track, kanban + list, filters city/tier/stage/owner), **Organisation**
  (people, timeline, compose, call), **Research** (queue + "new research run" brief + fit profile), **Pitches**,
  **Member updates**, **Coverage** (Europe map/table per track, territories of agents), **Templates**.
- Member updates read members from the existing accounts (`users.marketing_opt_in`, `locale`); new items come from
  the catalog / content store, so "new item on scutoons" can trigger a draft update.
- `server/`: `Domain/Embassy` (organisations, persons, leads, activities; later pitches, templates), `/api/embassy/*`
  behind the roles liaison / ambassador; admin gets the role toggle. Mail: SMTP send + IMAP poll (cron) per shared mailbox.
- AI: Claude API on the server (drafts, reply classification, enrichment with web search), every result is a
  suggestion stored with its sources, never written straight into the data.
- Two roles, and they differ only where the rights differ:
  - `liaison` (part time): sees all leads, works the leads assigned to them (calls, mails, notes); ~~their first-contact
    mails wait for approval~~ – changed 2026-10-04 (owner): anyone with the role sends, no approval step ("Ask + mail").
  - `ambassador` (full time): everything a liaison does, plus sending without approval, approving liaisons' first
    mails, assigning leads, owning territories/tracks, starting research runs, member updates and reporting.
  - owner/admin: everything, plus role toggles in the admin app.
- The public Top List (community resource) stays public and separate: private CRM fields never leak into it. The old
  Django/LitElement toplist plan is superseded by this.

Off-the-shelf alternative if time is short: folk, Attio, Pipedrive or HubSpot free do contacts + pipelines + mail
sync today. Building on the platform wins on: one login for everything, helpers on the same accounts, AI wired to our
own data, no per-seat fees. Phase 0 works either way.

## Phases

| Phase | What | Done when |
|---|---|---|
| **0 – now, no code** ✅ import file | owner: tiers, first pitch (EPK, release links), 3 first-contact templates (de/en); agent: clean + dedupe the Top List into an import file | import file reviewed |
| **1 – MVP** ✅ built | DB + API + `embassy/` app: import, organisations/persons/leads, territories, stages, next date, Today list with call tasks + outcome buttons, activity notes, roles; compose opens the mail app (`mailto:`) and logs it | owner + 1 helper work a week in it on the phone |
| **2 – mail** | SMTP send from the app, IMAP sync, reply stops sequence, cadence defaults (mail → call), daily reminder; member updates to opted-in members with unsubscribe | no manual logging needed; first member update sent |
| **3 – AI** | enrichment (incl. phone numbers), drafts, reply classification, **research runs** + fit profile, "who should hear about this pitch" | owner starts a research run in one sentence and gets candidates with reasons |
| **4 – scale** | scheduled research runs, Europe coverage view, reporting per helper/track, more languages, member update triggered by new catalog items | – |

## Open questions (owner)

1. ~~Priority~~ started now, on its own branch, merged after the release (2026-10-01).
2. ~~Name~~ Embassy, embassy.scutoons.com (2026-10-01).
3. Which other sources besides the Top List? (files, Instagram follows, past email threads, Bandcamp buyers?)
4. Which mailbox sends: a shared `booking@` / `hello@` mailbox, or each helper their own?
5. ~~Region~~ Europe (answered 2026-10-01). First pitch: which release, is there an EPK?
6. Who makes the calls: you, helpers, or both? Should calls happen from their own phones (tap to call, log the outcome)
   or through a shared number?
7. Member updates: how often at most (e.g. at most 2 per month)? Do they replace the existing marketing mails, or
   are they the first ones?
8. Is the Top List itself (public, Top Toy's community spreadsheet) to be moved onto the platform as a public page too?

## Built (2026-10-01, branch `embassy`)

- **API** (`server/`): migration `2026-10-01-embassy` (tables `embassy_orgs`, `embassy_people`, `embassy_leads`,
  `embassy_activities`; roles 4 `liaison`, 5 `ambassador`), `Domain\Embassy\Embassy`, `/api/embassy/*`
  (`server/docs/API.md` "Embassy"). Cadence: mail → call in 6 days; call reached / reply / meeting → in conversation,
  10 days; voicemail / no answer → 3 days; call back → chosen date; declined → `no`. Automatic dates skip weekends.
  Working an unowned lead (anything but a note) makes you its owner. Duplicates: same normalized name + same city.
- **App** (`embassy/`, port 13000): Today, Leads, organisation page, new organisation, import. de + en.
- **Import** (`scripts/embassy/toplist_import.py`): Top List xlsx → `C:\Development	oplist\embassy-import.json`
  (outside git, it holds contact data): 127 organisations, 63 people, 115 with a country. Vital Weekly's 38 writers
  are one organisation; Powerline's 11 bookers are people; duplicates (Cashmere, Noods, LYL, Raheem, HKCR) merged.
- Tested: PHP unit + contract suites, `npm run check`, builds, deploy dry run, browser walkthrough as
  `agent@scutoons.test` (import of the Top List into a local DB copy, mail → call → voicemail, duplicate warning,
  phone width 375 px, German).

**To go live (owner):** subdomain `embassy.scutoons.com` on all-inkl → folder `embassy/dist/` (+ SSL); run
`server/migrations/mysql/2026-10-01-embassy.mysql.sql` then `2026-10-02-embassy-stores.mysql.sql` in phpMyAdmin (the api deploy adds the origin to `ALLOWED_ORIGINS`); grant roles in the admin app (Users); import the file on `/import`
(dry run first). Local: nothing to add, the dev API allows every port of `shared/dev-ports.json` (PLATFORM.md "Local dev ports").

**Released with the 2026-10-04 rollout (owner 2026-10-04; was held back since 2026-10-01):** `embassy` is in
`deploy/targets.json` `order`, so the Deploy workflow ships it on pushes to main; `.github/workflows/embassy.yml` stays the
manual single-app run. The DB migrations are additive.

**Next (phase 2):** sending from the app (SMTP, shared mailbox), inbox sync, templates + call notes, member updates
to opted-in members, daily reminder. Then phase 3 (AI enrichment, drafts, research runs).

## Stores – "Where to buy" (2026-10-02, owner — built on branch `stores`)

Owner: "In the embassy I need a type STORE. Any store that carries products from SCUTOONS needs to be mentioned. We need
a public page on the main viewer which lists which stores carry which items, so people can buy from a local supplier of
their choice. I want to encourage small vendors at first."

**Model** (on `embassy_orgs`, migration `2026-10-02-embassy-stores`, sqlite + mysql, additive):
- New organisation type **`store`**. `record_shop` stays. Both are **stockists** (`Embassy::STOCKIST_TYPES`,
  `@scutoons/api` `STOCKIST_TYPES` / `isStockist`): they sell to readers over the counter, a record shop being a store
  that specialises in records (the Vinyl track's targets, many came with the Top List import). Distributors and labels
  sell to shops, not to readers, so they are not listed (open decision below).
- Store fields, edited in the organisation form when the type is a stockist: `listed` ("Auf scutoons.com zeigen"),
  `independent` (small / independent vendor, listed first), `address`, `opening` (free text as the store writes it),
  `carries` = product ids of the shop catalog (table `catalog_products`; unknown ids → `400 invalid_product`). Kept
  on the row when the type changes, but only stockist types are public. The import unites `carries` like the tags.
- No separate table: a store is an organisation like every other partner (people, leads in the Vinyl track, timeline);
  the public page only reads the five store columns plus name / type / city / country / website.

**Public read**: `GET /api/stockists/?product=&country=` (`Domain\Embassy\Stockists`, `StockistsController`; server
docs/API.md "Embassy → Stores"): listed stockists with at least one product still in the catalog, public fields only
(`id, name, type, independent, city, country, address, website (http(s) only), opening, carries`) – never people, email,
phone, notes, tags, tier, source, leads or activities. Order: independent first, then country, city, name.
`products` (product id → number of stores) and `countries` cover all listed stores (unfiltered), so one request answers
"is it in a shop?" for every product. No session, any origin, `Cache-Control: public, max-age=300` + ETag (304), 600
requests / IP / hour (`stockists:<ip hash>` bucket). Before the migration ran: an empty list, no error.

**Embassy UI**: organisation form → "Laden / Store" fieldset (only for store / record shop): the two switches with
tooltips, address, opening info, the products as checkboxes with their icons (`/products/`, CDN icons: the embassy build
now gets `VITE_CDN_URL`); the organisation page shows a store box (LISTED / NOT LISTED, INDEPENDENT badges, a warning
when listed without products, the products, a link to scutoons.com/stores).

**Viewer** (scutoons.com): `/stores` (`/en/stores`), title "Wo kaufen" / "Where to buy": "Unabhängige Läden" first, then
"Weitere Läden", each grouped by country (name from the browser, `Intl.DisplayNames`) and city; per store the name, type,
"unabhängig" tag, address, opening info, the products (icon + catalog name) and the website. Filters product + country
live in the URL (`?product=`, `?country=`). Empty states: no store at all → "Online kaufen" (the shop); no store for a
product → "Online kaufen" (the product's Bandcamp link) + "Alle Läden zeigen". "Läden vor Ort" / "Local stores" next to
"Kaufen" in the toon page's related row (`related-items.svelte`) and under the landing page's editions whenever a listed
store carries that product; "Wo kaufen" in the main menu and the landing footer – both only once at least one store
is listed (no empty page in the menu at the release). Texts in the messages (de/en).

**Dev data**: `php server/bin/console sample-stores [--remove]` adds three clearly fake stores to the local dev database
("Testladen (Beispiel)" Wien, "Beispiel-Plattenladen" Berlin – both independent –, "Testkette Megastore (Beispiel)" Graz),
source "dev sample". Only on a local database named `scutoons…` (production's is not); nothing is in any production seed.

**Tests**: PHPUnit `EmbassyTest::testStoreFieldsAndTheStockistList` (fields, validation, order, filters, unlisting,
import merge, public keys only), `EmbassyApiTest::testStockistsArePublicAndCarryNoPrivateFields` (no session, CORS *,
cache, 304, no private strings, filters, 400s); api-smoke "Stockists" (12 checks; suite removed in CUTOVER 10.30); `npm run check:stores` (headless,
390 / 1440 × de / en: the list, independent first, filters, empty states, the "Local stores" links, menu); check-landing
expects the new footer link; check-toon-hero still green (the related row's buttons now wrap).

**Open owner decisions**:
1. What counts as "small / independent"? Today a checkbox the liaison sets by judgement (no size field). A rule (no
   chain, owner-run, under N branches) would make it consistent.
2. Should non-independent stores be listed at all for now ("encourage small vendors at first" could also mean: only
   small vendors in the beginning)? Today they are listed after the independent ones.
3. Do stores apply themselves (a public "list my shop" form → a research lead in the Vinyl track), or does only the
   Embassy add them? Not built.
4. Distributors / labels / mail-order with a web shop: list them too (a third block "online")? Today no.
5. A map view and "near me" (geolocation, coordinates per store) – later; today grouping by country and city.
6. Per-store stock (sizes, variants, sold out) and a deep link per product in the store's web shop – today only "carries".
7. Opening info is free text in the store's own words (not translated) – fine, or structured hours?

## Ask + mail (2026-10-04, owner — built on branch `ask-shared`)

Owner: the Ask agent in Embassy helps communicate with the clientele (send out new releases, track communication);
start with basics, and make Ask a shared integration of embassy, creator and admin. Decisions (2026-10-04): mails are
**sent over SMTP from the app**, from **hello@scutoons.com**; **anyone with the role** (liaison / ambassador / admin)
sends – no approval step; replies are **read back by IMAP** in this slice; what gets pitched are the **catalog's albums
and products** (no new "pitch" record yet). The agent only **drafts**: a person reads the draft and presses Send.

- **Ask panel** (`@scutoons/ask`, shared/ask): round message button in the top bar, the outermost region of the
  embassy's AppShell (agents/DOCKING.md: the page makes room and stays centred, its edge drags it wider), full screen on
  phones. On an organisation page the agent knows which one (name shown,
  id sent). Tools (tools/mcp `src/embassy.ts`): `today`, `search_orgs`, `read_org` (incl. the log and `recipients` =
  addresses that may get mail), `list_team`, `list_releases`, `log_activity`, `set_lead`, `create_org` (a new organisation + its people and leads; duplicate → the existing id), `update_org` (only the fields named: contact details, social `links`, do-not-contact, store fields), `add_person`, `update_person`, `delete_person` (ambassadors), `draft_mail`, `check_inbox`. Social media is a valid channel (owner 2026-10-07), above all without a mail address: the agent writes the DM in its answer, the person sends it from our account, then `log_activity` kind `dm` (→ contacted, check in 6 days).
  The system prompt carries the "Communication strategy" above (individual first contact, their language, signed with
  their own first name, one ask, one link, never invent facts, calls for follow-ups).
- **Draft card** (`embassy/src/lib/components/mail-draft.svelte`): `draft_mail` comes back as a card (to, subject,
  text – editable); Send → `POST /api/embassy/orgs/{id}/mail`. The same card on the organisation page: "mail sent" in
  the log form = write the mail there and send it ("only log it" for a mail sent elsewhere). A sent card stays "Sent and
  logged" (remembered per browser), and the open organisation page reloads on every change the agent makes.
- **Sending** (API): one-to-one, plain text, our own Message-ID (threads), the opt-out line appended by the server,
  do-not-contact checked (409), logged as `mail_out` (subject, text, to) → contacted, call in 6 days.
- **Replies** (API, IMAP): `POST /api/embassy/inbox/sync` when Today or an organisation page opens (at most every
  2 minutes), `php bin/console embassy:inbox` for a cron job. Matched by thread, else by sender address; unknown
  senders are skipped and nothing about them is stored. A reply → `mail_in` (in conversation, 10 days: the follow-ups
  stop); a reply that is just "stop" / "Stopp" / "unsubscribe" sets do-not-contact.
- **Answered before, quiet now** (owner 2026-10-04: "keep track of dormant contacts that have been responsive in the
  past"): `GET /embassy/orgs?quiet=1` (or `quiet=<days>`, default 90) = leads with a reply, meeting or reached call
  and then nothing for 90 days, no date ahead, not "no", not opted out. Leads checkbox "answered before, quiet now",
  palette entry, and the agent's `search_orgs` `quiet`.
- **Pace + weekly portion** (owner 2026-10-05: "consumable portions per day or per week … I need to outsource this and
  I want liaisons to stay on their own pace"; migration `2026-10-05-embassy-pace`): every liaison / ambassador sets
  contacts per week (preset 10; 5/10/15/20 or any 1–100) and a reminder (Mondays – preset, every working day, none) in
  an **onboarding** on their first visit, changeable on Today any time. The onboarding also asks their **first name**
  (required; owner 2026-10-07: mails are signed with the writer's first name) – the account's `display_name`
  (`PUT /me/`, the same name readers see); Today asks for it when it is missing and edits it with the pace. Ask gets it
  as "Writing: …" in every Embassy message and signs with its first word. **Today (mine)** = this week's portion: "X of Y"
  bar, today's slice first (what was left this morning spread over the working days left), then later this week, what
  doesn't fit waits for next week. Order: due calls → answered before, quiet now → first contacts (research/ready,
  never contacted). Each card says why it's there (`hint`) and has an Ask button (prepare the call / draft a mail);
  "Go through my week" asks the agent for one line per contact (AI only on request). Every **4 weeks** one question "is
  your pace right? less · fine · more" (−5 / 0 / +5). Ambassadors / admins see a **team table** on Today → everyone
  (pace, done per week over 4 weeks, contacts, last check-in). Leads a liaison adds are theirs. **Reminder mail** (to
  the team member only, from no-reply@): the week's portion (daily: today's slice), sent by the **cron URL**
  `/api/embassy/cron?key=EMBASSY_CRON_KEY` (all-inkl KAS → Tools → Cronjobs, daily 07:00), which also reads the
  inbox; once per week / day, never with nothing to do. Agent tool `my_portion`.
- **Not yet:** templates per track × language (the agent follows the strategy instead), a "who should hear about this
  release" list across leads, member updates, the legal opinion on automated follow-ups (see "Legal" above – nothing
  here sends on its own).

## Security note

`C:\Development\toplist\.windsurf` contains a database login + password in plain text. Rotate that password if the
folder was ever shared or committed, and keep secrets in a gitignored `.env` from now on.
