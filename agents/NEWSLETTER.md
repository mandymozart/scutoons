# NEWSLETTER – mass mails to members and contacts

Built 2026-10-07 (owner: "send emails to all registered members but also to people who left their email when claiming
their qr code … bundle to not overload the smtp"; first use: the invitation to the next chapter release, the week of
2026-10-12). Code: `server/src/Domain/Newsletter/`, admin app → Emails → Newsletters / Contacts / Unsubscribed. API:
server/docs/API.md "Newsletter". Settings: server/docs/CONFIGURATION.md (`NEWSLETTER_*`).

## Who can be mailed (owner decisions 2026-10-07)

| Audience | Where from | Consent |
|---|---|---|
| `members_opted_in` | `users.marketing_opt_in = 1` ("Send me news about new releases") | the checkbox (+ `marketing_opt_in_at`) |
| `members_all` | every account | the account – use for news that concern every member |
| `contacts:qr-claim` | the old QR form: `qr.email` of codes in no account, addresses without an account (copied by the migration) | the old form said a claim brings updates about the platform (owner); keep a screenshot of its wording |
| `contacts:event:<…>` | the paper sign-up lists of events, typed in by the owner (Contacts → Import) | the paper sheet (keep it); `consented_at` = the event day |

Every mail has an unsubscribe link and the one-click header, also for the old QR addresses (owner). Each address gets a
newsletter once; a member's account language and name win over its contact row. The owner sets the language of every
contact by hand (the list is small) – a newsletter can't start while a recipient has none.

## Contacts

Table `mail_contacts`: email (unique, lower-case), name, locale (`de` | `en` | NULL), source, consented_at. Import: one
address per line, optional name and de / en in any order, separated by comma, semicolon or tab. "Check" shows every
line's status (new · already a contact · has an account · unsubscribed · twice in the list · no valid address); only the
new ones are added. Members are never imported (they are mailed as members); an unsubscribed address isn't either
(remove it from Unsubscribed first – only when the person asked again).

## Writing a newsletter

Admin → Emails → Newsletters → New newsletter (a title only the admin sees). Then: who gets it (the audiences with their
counts; "N without language" links to the contacts to fix), subject + text in DE and EN, a live preview (the frame of
every platform mail), Save, Test to me (the saved text to the signed-in admin, subject `[Test] …`), Start (asks with the
exact number of recipients).

The text is plain: a blank line starts a paragraph, single line breaks stay; a paragraph that is only
`[Button text](https://…)` becomes a button, `![description](path)` a picture (a CDN path or a URL); `{name}` = the
recipient's name (empty without one), `{name|there}` = the name or "there" – also in the subject. Under the text every
mail says why the person gets it and links to unsubscribe.

**The sign-in button** (owner 2026-10-20, #65: one click = signed in, the text per recipient): a paragraph that is only
`{signin}` (the placeholder list inserts it) becomes a personal button (`NewsletterSignin`):
- an account → "Anmelden" / "Sign in"; a staff account (any role beyond member) → the same text, but a plain link to
  /account (a forwarded mail must never open an admin's account)
- no account, a QR code of the old form waits for the address (`legacySignupCode`) → "Account anlegen – dein QR-Code
  wartet" / "Create your account – your QR code is waiting": signing in creates the account and claims the code (a code
  with a choice of items opens its claim sheet instead)
- no account and no such code (e.g. event contacts) → no button, the paragraph is left out

The link `APP_URL[/en]/?signin=<token>` is a login request made when the mail is sent: single use, valid
`NEWSLETTER_SIGNIN_DAYS` (14), no 6-digit code, not counted for the hourly login limits (`ip_hash` =
`NewsletterSignin::IP_HASH`). The viewer asks once ("Bei SCUTOONS anmelden?" – mail scanners open links) and signs in;
used or expired → "Der Link geht nicht mehr" → the sign-in page. Preview: a member's button to /account; Test to me:
the admin's own (staff → /account).

**The gift** (owner 2026-10-20, #66; agents/research/CAMPAIGN-GIFTS.md): the editor's "Geschenk" picks a campaign of the
kind "Verschenken" (Campaigns – item, stock, window, countries live there), and a paragraph `{gift}` becomes the
recipient's button "Ein Geschenk wartet auf dich": the #65 sign-in link + `&gift=<token>` (staff: `?gift=` alone). Members
sign in, QR owners sign up with their product code, and recipients who gave nothing but their email get a personal promo
code (`campaign_codes`, one per campaign + address, ending with the campaign, then the campaign's reading rule) – the
editor lists the three groups with their counts. The viewer opens the gift sheet: "Abholen" (once per address, while the
campaign runs and its stock lasts); a physical gift waits for the account's shipping address (account → Stammdaten).
Start refuses a `{gift}` line without a gift and a campaign that is over or stopped. One campaign may serve several
newsletters (a reminder).

States: draft (everything changes) → sending (recipients fixed at the start – later contacts are not added) ⇄ paused
(subject and text can still be fixed; the rest gets the new text) → done. Draft and done can be deleted (mail_log keeps
its rows).

## Sending

`NewsletterSender::batch()` – from the host's cron (`/api/newsletter/cron?key=NEWSLETTER_CRON_KEY`, every 5 minutes) or
"Send now" on the Newsletters page (batch after batch while the page is open). One batch:

- runs alone (MariaDB `GET_LOCK`), at most `newsletter_batch` mails (runtime setting, admin → Settings, 1–25, default
  `NEWSLETTER_BATCH` 20 – 20 every 5 minutes = 240 an hour, so the hourly limit decides), at most `NEWSLETTER_SECONDS` (25 s)
- never over the hourly limit: runtime setting `newsletter_per_hour` (admin → Settings, default 200, owner
  2026-10-07: "start with 200"). Every mail of the last hour counts (`mail_log`), login codes too – so a sign-in never
  waits behind a newsletter more than the limit allows
- one SMTP connection and login for the batch (`Smtp::open/deliver/close`, RSET between mails), a new one every
  `NEWSLETTER_PER_CONNECTION` (50), `NEWSLETTER_DELAY_MS` (1 s) between two mails
- headers `List-Unsubscribe: <https://api.scutoons.com/api/newsletter/unsubscribe?t=…>` and
  `List-Unsubscribe-Post: List-Unsubscribe=One-Click` (Gmail / Yahoo bulk-sender rules)

At 200 / hour: 1 000 recipients take about 5 hours. Ask All-inkl for the mailbox's real limit before raising it.

## Unsubscribe and bounces

Table `mail_suppressions` (email, reason, detail). Every newsletter skips these addresses, also ones added after its
start (status `skipped`). Login and other service mails are never suppressed.

- `unsubscribe`: the link (a page with one button – a GET changes nothing, mail scanners open links) or the mail
  client's own button (one-click POST). A member's opt-in goes off too; ticking it again on the account page lifts the
  unsubscribe.
- `bounce`: the SMTP server refused the address for good (5xx at RCPT TO). Typos from handwritten lists end here – they
  hurt the sender reputation if mailed again.
- `manual`: blocked by an admin (Unsubscribed → Block an address); "Mail again" removes any of them.

Other failures: a temporary one (4xx, a broken connection after the login) is tried again after
`NEWSLETTER_RETRY_MINUTES` (15), failed after `NEWSLETTER_ATTEMPTS` (3). A problem with the account or the server
(connection, TLS, login) stops the batch at once, counts nobody's attempt and shows on the newsletter (`last_error`).

## Going live (done – owner 2026-10-12; the steps as they were)

1. The migration `2026-10-07-newsletter` (applied; new migrations run with the deploy).
2. GitHub → Settings → Secrets → Actions: `NEWSLETTER_CRON_KEY` = a new random string (`openssl rand -hex 32`).
3. Push `main` (deploys api + admin).
4. All-inkl KAS → Tools → Cronjobs: URL `https://api.scutoons.com/api/newsletter/cron?key=<the key>`, every 5 minutes.
5. Check SPF / DKIM / DMARC of scutoons.com in KAS (DNS); send a test to a Gmail, a GMX / web.de and an Outlook address.
6. Admin → Emails → Contacts: set the language of the `qr-claim` contacts, import the event lists.
7. Write the newsletter, Test to me (de + en), Start.
