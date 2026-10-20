Status 2026-10-20: concept, owner answered §8 1–3, question 4 open (#66) – nothing of it is committed; a first newsletter-only build lies unused in the working tree (§9).

# CAMPAIGN-GIFTS — SCUTOONS gives an item away: one campaign, several ways to hand it out

Owner 2026-10-20: "kann ich … links generieren, dass ich wenn ich email als newsletter verschicke, dann ich digitale
items verschenken kann? oder spaeter sogar physikalische items?" – decided: "abholen. ein geschenk wartet auf dich.
stueckzahl grenze. ich kann es festlegen beim giften. haengt aber vom item ab. manche sind 1 per account / verfaellt
richtig. / physikalische brauchen die versand adresse. die muss vom reader im account nach getragen werden." Then:
"genau ich habe campaigns. die muessen damit verbunden werden … oder wir brauchen ein unifying concept" and "bei
campaigns [werden] auch neu codes erzeugt … das darf dann nicht zu verwirrend werden".

## 1. The idea in one line

**A campaign = SCUTOONS gives something away in a time window.** What it gives (item, stock, countries) and when (from /
until) is the campaign. How it reaches people is the campaign's **kind** – today members invite friends; new: the
gift goes straight to the recipients of a newsletter. Same page in the admin, same numbers, same "to send" list.

## 2. The two kinds

| | **Einladen** (`gift_invite`, built 2026-10-14) | **Verschenken** (`giveaway`, new) |
|---|---|---|
| who gives | a member (their own item or the campaign's) | SCUTOONS |
| who gets it | a friend **without** an account | the recipients of a newsletter (later: other channels, §7) |
| how it arrives | the member types an address → an invite mail → the gift waits until the friend signs up | the newsletter's button "Ein Geschenk wartet auf dich" → the reader claims it ("Abholen") |
| limit | per member, per day, stock | stock; per account: the item's own limit (`max`, e.g. 1) |
| after the window | not joined → back to the member | not claimed → it lapses ("verfällt") |
| physical | the friend gives an address per invite | the reader's **account address** (§5) |
| new codes | **yes**: one promo code per invite (the friend's way in) | **none** (§3) |

One campaign has one kind (the admin picks it when creating it; it can't change once something went out).

## 3. Codes – which one comes from where (so it doesn't get confusing)

- **Product codes** (QR on a record, shirt, amulet): printed, unchanged. They are how people own things and sign up.
- **Einladen** makes **one promo code per invite** – only so a friend without a product can make an account. It works
  once and ends with the invite. The admin never handles these codes; they show only as "Einladungen" on the campaign.
- **Verschenken makes no codes.** The newsletter button is a personal *link* (signed with the address, single use for
  the sign-in part, #65) – not a code. Who can receive:
  - members: sign in with the link, claim
  - old QR-form contacts without an account: the link signs them up **with their own existing product code** (#65), then they claim
  - contacts without an account and without any code (event lists): no button – an account still needs a code
- **A physical gift** is shipped; the product in the parcel carries its own printed QR code, claimed the normal way.
  Verschenken itself still adds no code.

So the code list (admin → Codes) only ever grows by printed product batches, promo batches and invite codes – never by
a giveaway.

## 4. What the admin does

**Campaigns → New campaign → kind "Verschenken":** name, from / until (= the claim window), the item (+ variant),
stock (empty = no limit), for a physical item the countries it ships to. Start / stop as today.

**The campaign page** (as today, by kind): numbers – claimed / left; for physical gifts the list "Zu versenden"
(name, address, country, claimed on) with **Versendet**, and "Wartet auf Adresse" (count). A started campaign keeps its
item (things went out).

**Newsletter editor:** a select "Geschenk" with the running and planned Verschenken campaigns (none = no gift); the
placeholder list offers `{gift}` (a line of its own, like `{signin}`). Start refuses a `{gift}` line without a campaign,
and a campaign whose window ends before the start. The newsletter keeps nothing of the gift itself – only which
campaign.

## 5. The reader

- **The mail:** "Ein Geschenk wartet auf dich" / "A gift is waiting for you" (button) → the viewer asks once to sign in
  (#65) → a sheet: the item, "bis <date>", **Abholen**. States: already yours · the time is over · all given away ·
  "signed in with another address".
- **Virtual:** into the inventory at once (source `gift`).
- **Physical:** "Kommt per Post – trag deine Versandadresse ein" → account → Stammdaten → **Versandadresse** (name,
  address, country). Saved → the gift moves to the admin's "Zu versenden". Without an address it waits; after the
  window it still waits (it was claimed in time) – only unclaimed gifts lapse.
- The account address is the reader's own, editable any time; the admin sees it only on gifts to send.

## 6. Data (migration `2026-10-2x-campaign-giveaways`)

- `campaigns.kind` gets the value `giveaway` (inviters / per_member / per_day / wait_days / access stay unused for it).
- `campaign_claims`: campaign_id, email (the address it was sent to), user_id, item_id, variant, status `claimed |
  address | ship | shipped`, ship_name / ship_address / ship_country (copied when it ships), created_at, shipped_at;
  UNIQUE (campaign, email) = once per recipient.
- `mail_campaigns.gift_campaign_id` → the campaign (NULL = no gift).
- `users.ship_name / ship_address / ship_country` – the account address.
- Stock = the campaign's `stock` minus its claims (giveaway) / minus its invites (gift_invite), as today.

## 7. Later (not now)

- More channels for one giveaway: a public link or a QR on an event poster (the same claim sheet; with a per-person
  limit). Only when asked.
- Einladen may use the account address too (the friend's per-invite form becomes "the account's address") – one address
  per reader everywhere.
- Paid physical gifts (Einladen's "member pays") stay Einladen-only.

## 8. Owner answers (2026-10-20) and the one still open

1. Names "Einladen" / "Verschenken" – yes ("in deutsch passt das").
2. One campaign, several newsletters (a reminder) – yes; the claim stays once per address.
3. A claimed physical gift without an address lapses 30 days after the window ends – yes (a reminder on the account page
   meanwhile).
4. **Open – who of the recipients can take part** ("bitte mal die faelle auflisten. das muessen wir abwegen"):

| # | Recipient | What the button does | Note |
|---|---|---|---|
| A | member (role member only) | signs in with one click (#65), "Abholen" | the main case |
| B | staff account (admin, creator, …) | opens the viewer; they sign in themselves, then "Abholen" | no one-click sign-in for staff (#65) |
| C | old QR-form contact, no account, their product code still free | **creates their account** with that product code (the code is claimed into it), then "Abholen" | the account is made by the newsletter click – the same as signing up with that email on the account page today |
| D | old QR-form contact whose code was claimed by someone else meanwhile, or expired / revoked | no button | nothing to sign up with |
| E | event-list contact (no account, no code) | no button | an account needs a code; giving them one = a new promo code per person (what §3 avoids) |
| F | a contact who made an account with the same email meanwhile | as A | member data wins (NEWSLETTER.md) |
| G | a contact who has an account under **another** email | as C/D for the contact's email – C would make a **second account** | we can't know the two belong together |
| H | a forwarded mail | the one-click sign-in works once for whoever clicks first (14 days); the gift belongs to the address, so only that account can claim it | the sign-in risk exists since #65 |
| I | a second newsletter for the same campaign (reminder) | "Schon deins" when claimed already | answer 2 |
| J | C, but the product code offers a choice of items | the code's claim sheet first (pick the item), then the gift | rare (#65) |

Ways to decide 4:
- **a. members only (A, B, F):** simplest; QR owners without an account get the gift only after they sign up some other way.
- **b. members + QR owners (A–D, F–J)** – recommended: the gift is the reason to finally make the account; risk G (a
  second account) is small and the same as today's sign-up page.
- **c. b + event contacts (E)** with a personal promo code each: reaches everyone, but a giveaway then creates codes
  again – exactly what the owner wants to avoid.

## 9. Where the first build stands

Built this session before the owner asked for campaigns (working tree, not committed): the claim logic (once per
address, stock, window, the item's per-account limit, physical → address → ship → shipped), the `{gift}` button with the
#65 sign-in, `ShippingAddress` + `GET/PUT /api/me/shipping`, admin endpoints, 4 tests. It hangs the gift on the
newsletter (`mail_campaigns.gift_*`, `mail_gift_claims`); after this concept it moves to the campaign (§6) – the logic
stays, the place changes. Not built yet: the admin and viewer UI.
