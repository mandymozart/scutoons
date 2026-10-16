Status: built 2026-10-16 as concept B (owner: "Stil Entwurf B mit den Menüs") – how it is now: DESIGN.md "Account page".
Owner's changes to the draft: no group "Konto" ("Konto Konto klingt komisch") → "Verbindungen" (AI) + "Stammdaten"
(name first, address, sign out, roles, memberships, "Konto löschen" = contact form, privacy topic); icons before every
group; "Dieses Gerät" → "Offline-Inhalte"; one back link look on both levels. Not built from the draft: the in-chapter
quick panel (concept C). Push (PWA phase 4) sits in "Benachrichtigungen" (guests too: it is per device), the iOS
how-to with "App installieren" and the saved seasons in "Offline-Inhalte". The text below is the draft.

# ACCOUNT-PAGE — reorganising the reader's account page

Owner 2026-10-16: the account page (`viewer/src/routes/(account)/account/+page.svelte`,
`viewer/src/lib/components/account/*`) grew by appending; related things sit far apart. This doc lists what is on it,
groups it, plays three concepts through (contexts × states), covers accessibility and ends with open questions.

## 1. Inventory (today, top to bottom)

| # | Section / setting | What it does | Who sees it | Stored | Also elsewhere |
|---|---|---|---|---|---|
| 1 | "Zurück zu {toon}" link | `history.back()` to the page the reader came from | anyone arriving from an app page | – | logo, menu |
| 2 | "Mein Konto" heading + icon | – | all | – | menu entry "Mein Konto" / "Anmelden" |
| 3 | Angemeldet als … · Abmelden | `SignedInAs` (@scutoons/auth); signs out of every app | member | – | – |
| 4 | Rollen / Zugang / Erstellt am (table) | roles, access ("Alle Kapitel · …", promo code, none), created date | member | account | – |
| 5 | Neuigkeiten-Opt-in | newsletter mails | member | account | – |
| 6 | Sensible Inhalte ausblenden | blurs marked panels, mutes them | member (guests: always hidden, no toggle) | account | the slice's "Sensibler Inhalt ausgeblendet" note links to `/account` |
| 7 | Hinweise zeigen | outlines tappable areas | member (guests: off, no toggle) | account | – |
| 8 | Creator (link) | to creator.scutoons.com | creator/admin | – | – |
| 9 | Entwurfsvorschau | read draft / coming-soon chapters | creator/admin | device | the eye mark top right links to `/account` |
| 10 | Karte zurücksetzen | the map as if read anew | creator/admin with draft preview on | account (all devices) | – |
| 11 | Mitgliedschaften (table) | codes · type · until | member | account | – |
| 12 | Meine Ansprüche | ownership claims: add proof, withdraw | member with claims | account | claim page |
| 13 | KI-Verbindungen | MCP tokens, setup steps | member (API says `member`) | account | – |
| 14 | Anmelden (LoginForm) + signup rule | email code sign-in; `?return=`, `?code=` | guest | – | membership blocker, inventory "Anmelden" |
| 15 | Zugang auf diesem Gerät | device code, until | anyone with a valid device code | device | – |
| 16 | Dein Name | `{reader.name}` in stories | all | account / device | a story's askText |
| 17 | Medienqualität | Auto / Niedrig / Mittel / Hoch + MB estimate | all | account / device | – |
| 18 | Soundeffekte | volume, Anhören, Vibration | all | device | – |
| 19 | App · App installieren | install prompt | browsers that can install (Android / Chromium) | – | the install bar |
| 20 | Kapitel auf dem Gerät speichern | Nur von Hand / Im WLAN / Immer | browsers with service worker | device | – |
| 21 | Gespeicherte Kapitel | running saves (bars per kind), list, remove, remove all, storage | when something is saved / saving | device | the chapter row's save button |
| 22 | Inventar | item count + "Inventar öffnen" | all | – | header's inventory button (panel) |
| 23 | Avatar + stats | the full avatar editor | all | – | header's avatar panel |
| – | *planned* Push notifications | PWA phase 4 (iOS only installed) | ? | ? | – |
| – | *planned* iOS "Teilen → Zum Home-Bildschirm" how-to | Safari, not standalone | iOS Safari | – | – |
| – | Sprache | de / en | all | cookie | **only in the main menu** (stays there) |

Findings:
- Three kinds of things are mixed: **settings** (16–21, 5–7, 9), **status / records** (4, 11, 12, 15) and
  **shortcuts to panels** (22, 23 – duplicates of the header buttons).
- Reading settings sit in two places: hints / sensitive at the top (account block), media quality / name below.
- Every setting repeats "Saved in your account / on this device" as a sentence – a small scope label would do.
- A guest sees the sign-in form first and the device settings below without knowing they apply only here.
- Gap: a guest's name stays on the device at sign-in (the account's empty `display_name` wins – `reader-name.svelte.ts`);
  media quality falls back to the device value, so it carries over by accident.

## 2. Groups

| Group (de / en, new texts) | Holds | Scope | Collapsible |
|---|---|---|---|
| top card (not a group) | guest: sign-in / join · member: email, access line · device code | – | no |
| **Lesen / Reading** | Dein Name, Medienqualität, Hinweise zeigen, Sensible Inhalte ausblenden | account (guest: device) | yes |
| **Ton & Vibration / Sound & vibration** | Lautstärke + Anhören, Vibration | device | yes |
| **Dieses Gerät / This device** | App installieren / iOS how-to, Kapitel speichern (auto), Gespeicherte Kapitel + bars + storage | device | yes (opens by itself while a save runs) |
| **Benachrichtigungen / Notifications** | Neuigkeiten per E-Mail, (later) Push | account | yes |
| **Sammlung / Collection** | Inventar, Avatar (both as links to their panels), Meine Ansprüche | account | yes |
| **Konto / Account** | Mitgliedschaften, Rollen, Erstellt am, KI-Verbindungen, Abmelden | account | yes |
| **Creator** | Creator link, Entwurfsvorschau, Karte zurücksetzen | mixed | yes; only creator/admin |

Each group's closed state shows a one-line **summary** of its values ("Automatisch · Hinweise an"), so most readers never
open it. A scope chip "Konto" / "Gerät" per group replaces the per-setting sentences.

## 3. Concepts

**A – one page, grouped disclosures.** The top card, then the seven groups as disclosure rows with their summary;
one tap opens one in place. Opened from a chapter: "Lesen" and "Ton & Vibration" open, a sticky "Zurück zu Kapitel 3"
bar at the bottom (`StickyActions`). Desktop: the same column (max 600 px today), the groups stay disclosures.
+ one URL, find-in-page, no routing, closest to today. − long on a phone once groups are open; from a chapter it is
still a page change (scroll position kept by history).

**B – settings index → sub-pages (mobile-app style).** `/account` = top card + one row per group (label, summary, ›);
each group is a sub-page (`/account/reading`, `/account/device` … new URLs, allowed) with "‹ Mein Konto" on top.
Desktop: master-detail (the index left, the group right). + short screens, deep links (the sensitive note can open
`/account/reading` directly), familiar. − two levels of back from a chapter; groups with two settings (Ton) make thin
pages; more routes to maintain.

**C – hybrid: quick panel in the chapter + the page (A) for the rest.** Inside a chapter the menu gets one row
"Einstellungen"; it opens the dark `PullPanel` over the chapter (the chapter keeps its spot) with what matters while
reading: Lautstärke, Vibration, Hinweise, Sensible Inhalte, Medienqualität, "Dieses Kapitel aufs Gerät laden", and
"Alle Einstellungen ›" (→ `/account?return=…`). The page itself is concept A. + no page change mid-chapter, reuses the
panel pattern, the page stays one view. − one more panel; the quick settings exist twice (same components).

States (all concepts): **guest** – top card = sign-in (email code) + "Noch kein Konto? … Code scannen"; Lesen shows
name + quality and one muted line that hints / sensitive need an account; Konto and Sammlung's claims are hidden.
**Guest with device code** – the card adds "Zugang auf diesem Gerät · Code …". **Member** – card = email + access line;
all groups. **Member without access** – the access line "Noch kein Code in deinem Konto" + "Code scannen" (the blocker's
text). **Creator / admin** – the Creator group last; the draft-preview eye still links to it.

Join flow (guest → member): the blocker's or the inventory's "Anmelden" → `/account?return=<chapter>` → email code →
back at the chapter. New: what the guest set on this device (name, media quality) is **written to the account if the
account has none**, with one line "Von diesem Gerät übernommen: Name, Medienqualität"; the account wins otherwise.
Sound and saved chapters stay device-only by nature.

## 4. Accessibility (all concepts)
- One `h1` ("Mein Konto"); each group an `h2` that **contains** the disclosure `<button aria-expanded aria-controls>`
  (not `<details>`, so the heading stays in the outline); the summary is part of the button's name.
- Landmarks: the page `main`; in B the index is a `nav` ("Einstellungen"); the panel in C is a `dialog` with a title.
- Focus: back link first; opening a group moves nothing; closing a sub-page (B) returns focus to its row; closing the
  panel (C) returns it to the menu toggle. Escape closes the panel.
- Every row / toggle at `--control-height` (≥ 44 px touch); checkboxes keep their label as the target; the segmented
  rows are radio groups (`fieldset` + `legend`, square, the chosen one inverted).
- Errors `role="alert"`, saves `role="status"` ("Gespeichert"); the progress bars already have `role="progressbar"`.
- Reduced motion: disclosures and the panel open without animation. rem only; round = standalone icon buttons,
  square = everything with text and grouped controls.

## 5. Recommendation
**C** (A + the quick panel). The page stays one view – what the owner liked about it, no new routes – and gets order
from the groups and summaries; the chapter gets the few controls a reader actually reaches for without leaving the
page. B is the familiar app pattern but costs a level of back navigation from the chapter and makes thin pages.

## 6. Open questions (owner)
1. Avatar and Inventar on the account page: links to their panels (proposed) or keep the full avatar here?
2. Guests and "Hinweise" / "Sensible Inhalte": stay account-only (proposed) or device settings for guests too?
3. Where does the quick panel open in a chapter: a menu row (proposed), or a sixth round button in the header?
4. Carry device settings into the account at sign-in (proposed: only where the account has none)?
5. Push notifications (planned): which group – "Benachrichtigungen" next to the newsletter (proposed)?
6. Collapsed by default everywhere, and remember open groups per device (proposed)?

## Drafts
Artifact (concepts side by side, phone + desktop, de / en, the states): https://claude.ai/artifact/AaCX8ecNkTCH2tDWdsG8b7
– source `agents/research/account-page-drafts.html` (open it directly in a browser too).
