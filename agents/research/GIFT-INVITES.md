Status 2026-10-14: built (branch campaigns, FEATURES 10.96 done) – how it is now: ITEMS.md "Gift invites"; this file keeps the decisions and their story.

# GIFT-INVITES — give an item to someone without an account: invite them (plan, 2026-10-13)

Owner 2026-10-13: "if they don't have [an account] … push a warning … this is not existing. Do you want to invite them?
… invite them with a promo code." Today a gift to an unknown address is refused (`404 recipient_not_found`, "ask them
to sign up first"; ITEMS.md "Gift rules") – no pending gifts, because signing up needs a product or promo code.
The invite carries that code, so the refusal becomes an offer.

## Decided (owner 2026-10-13)

- **The gift waits:** taken from the giver at once and reserved; delivered when the friend signs up; back to the giver
  if unused after 30 days.
- **A personal invite code:** a new single-use promo code per invite (§4), tied to that gift – it can't leak or be reused.
- **SCUTOONS mails it:** an invite mail naming the giver, with the code link; limited per giver per day and once per
  address (against spam).
- **Recall:** a gift can be called back ("Any gift has to be recalled") – only while it is waiting, not once the friend
  has it ("only if not accepted yet").
- **The invite's access ends** (owner: "i need to set a limit for the new promo code like promo ends date and time"):
  the invite code reads like a promo code, but only until an end date and time set in the admin; after that the friend
  needs a product like any new reader – their account, items and progress stay, and claiming a purchased product with
  the same email gives them that account back ("they get to reclaim that account").
- **Switched on in the admin** – not always on, a promotion tool ("cool for promotion … we can ask them to invite
  people, but only if the option is on"); "don't think it too complex".

## The plan

**1. The campaign (admin → Campaigns; owner 2026-10-14: "campaigns are a specially cool thing … we can also use
them for promo later, but now we need the most basic implementation").** A table `campaigns`: kind (only
`gift_invite` now – promo can be another kind later), name (admins only), runs from / until (members invite and
friends accept inside it; outside it the gift form behaves as today), **who may invite** (owner 2026-10-14: a list of
accounts with a checkbox each and a select-all box that selects every account across all pages – also members who
join later; "members who want news" is no criterion; member groups / segments can plug in here later), **invitees' reading**
(*keep reading after the campaign*: whoever joined during it keeps access, overriding the invite code's end – the
owner's twist; or *until* a date and time, then they need a product), limits (gift waits 30 days but never past the
end, invites per member per day, one waiting invite per address). **Statistics** from `campaign_invites` (campaign,
inviter, address, item, invite code, state waiting · joined · back · called back, the account that joined, times):
members who invited, invites sent, joined, waiting, back / called back, per member; "joined later" = the address made
an account afterwards without the invite. Drafts: https://claude.ai/artifact/6jWjnwmb6sc5Yy8QCGuwuv (private).

**1a. The campaign gift (owner 2026-10-14: "campaigns which ask the user to invite and they are given a special item
as a gift … choosen from the items catalog").** No invite without an item. A campaign's gift is either *the member's
own item* (as above) or *a campaign item* chosen from the catalog – SCUTOONS gives it with every invite, the member's
inventory stays untouched; limited by the campaign's stock (invites stop when it is gone) and per member. **Physical
items** need shipping to the friend (owner: "special rules apply … scutoons can either pay for the shipping and article
or the person sending the gift can pay"): SCUTOONS pays → the friend enters a shipping address when accepting, the
order lands in a "to ship" list in the campaign, countries limited by the campaign; the member pays → by hand, no payment provider
(owner 2026-10-14: "a manual email send to the admin hello and ask them to send a money order link to the user, or a
bank account"): the gift is on hold ("payment open", the campaign's price incl. shipping), hello@scutoons.com gets a
mail, an admin sends the member a payment link or the bank details and marks the payment received in the campaign's
Invites tab – then the invite goes out; not cleared within 30 days, the gift ends. The first version includes physical items
(owner 2026-10-14).

**1b. Segments, account filters, promo batches (owner 2026-10-14, in the first version).** Account filters for the
Users list and a campaign's account list: signed up with (product / promo / invite code), owns or signed up with a code
of an edition or a batch, invited / joined in a campaign, in a segment. Segments = named groups assigned by hand
(Users & roles: tick accounts → Add to segment). A campaign's inviters: all · chosen · a segment; "all matching" ticks
every account the filters find (every page). Promo codes get batches like the editions' (qr.batch; a promo batch has
a name instead of an edition; the runner takes the code type from the batch).

**2. Giving (viewer, the gift form).** The giver enters an address without an account:
- switch off: today's message ("ask them to sign up first");
- switch on: a warning in the form, not an error – "There's no SCUTOONS account for this address yet. Invite them? We
  send them an invite with a code; the gift waits for them for 30 days." – and the button turns into **Invite and
  give**. Confirmed: the item leaves the giver's inventory, a notice says it is waiting.

**3. The waiting gift (server).** A table `pending_gifts` (id, giver, email, item, variant, qty, code, invite code,
created / expires / delivered / returned / recalled at). The item is taken from the giver like a gift today; a
code-bound item (a physical product's twin) stays with the giver's account until delivery (the code's owner can't be
an account that doesn't exist yet). Limits: it counts towards the giver's 10 gifts a day; one waiting invite per
address at a time.

**4. The invite code = a promo code (owner 2026-10-14: "is it really different to promo?" – no).** A code type only
says signup / claimable / grants, and an invite's are promo's. What is special lives in the invite row, per code: used
once (the invite knows it), accepted only while its invite waits (30 days, never past the campaign's end), and the
reading time = the code's own `expires_at`, set when the friend joins – none with "keep reading", the date with "until"
– so the existing access logic (`Permissions::access`, `AccountService::signupCodeRow`) needs no new rule. Created by
the server, one per invite; the admin's Codes list shows the link ("promo · invite from …").

**4a. When the access ends – already how accounts work.** An account reads with its signup code only while that code
is valid (`AccountService::signupCodeRow`, `Permissions::access`: an expired code grants nothing); the account itself
stays and signs in without a code. A product code claimed later with the same email lands in that account and unlocks
it again – the "reclaim" needs nothing new. To build: the reader's message when the invite's access has ended ("Your
invite has ended – a SCUTOONS product unlocks the stories again; your account and items stay").

**5. Signing up.** When the account is created (`LoginService::confirm`), every waiting gift for this address is
delivered – also when they sign up with another code – through the normal gift path (transfer log kind `gift`, the
recipient's "X gives you Y" notice and mail).

**6. Mails.** `gift_invite` (to the friend: who, what, the code, the link), `gift_returned` (to the giver: not
collected in 30 days, back in your inventory). The giver sees the delivery as a notice in the reader.

**7. Return after 30 days.** A daily cron URL like the embassy / newsletter ones (`/api/gifts/cron?key=…`) returns
expired waiting gifts and mails the givers; the giver's own requests also sweep their expired ones (no cron set up yet
→ still correct when they look).

**8. Recall.** In the giver's inventory a waiting gift shows as *waiting for <email>* with **Call back**: the item
comes back, the invite code stops working.

## Open questions for the owner

- ~~Q1 Recall after delivery~~ – answered: only while waiting.
- ~~Q2 Who may invite~~ – answered: ticked accounts in a list, select-all = every account (segments later).
- ~~Q4 Member gifts: accept first?~~ – answered: no, only invite gifts wait; member gifts stay instant and final.
- ~~Q5 Invite without an item?~~ – answered: no; a campaign can supply the item (1a).
- ~~Q6 Physical campaign items in the first version?~~ – answered: yes, the first version includes them.
- ~~Q3 What the invite unlocks~~ – answered: like a promo code, until the admin's *access ends*.
