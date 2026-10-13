Status 2026-10-13: plan; Q1 + Q3 answered, Q2 open (assumed: every member); nothing built (FEATURES 10.96).

# GIFT-INVITES — give an item to someone without an account: invite them (plan, 2026-10-13)

Owner 2026-10-13: "if they don't have [an account] … push a warning … this is not existing. Do you want to invite them?
… invite them with a promo code." Today a gift to an unknown address is refused (`404 recipient_not_found`, "ask them
to sign up first"; ITEMS.md "Gift rules") – no pending gifts, because signing up needs a product or promo code.
The invite carries that code, so the refusal becomes an offer.

## Decided (owner 2026-10-13)

- **The gift waits:** taken from the giver at once and reserved; delivered when the friend signs up; back to the giver
  if unused after 30 days.
- **A personal invite code:** a new single-use code per invite (code type `invite`: allows signup, unlocks like a promo
  code), tied to that gift – it can't leak or be reused.
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

**1. The switch (admin, the campaign).** One setting in the admin: *Invites with gifts* – off / on, with an optional
*from* and *until* (when invites can be sent; empty = open-ended) and the required **access ends** date and time (when
every invite code stops reading). Outside the window the gift form behaves as today. Stored as
one settings row (no campaign table, no campaign names, no stats page). A campaign with a name, its own limits or a
report can come later if a promotion needs it.

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

**4. The invite code.** A new code type `invite` in the catalog's code types (signup: yes, grants `catalog` like promo,
not claimable); one code per pending gift, created by the server (not the admin form), its `expires_at` = the switch's
*access ends*. Used once: signing up with it delivers the gift; afterwards no new account can use it.

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
- **Q2 Who may invite while it is on:** every member, or only accounts the admin marks (e.g. ambassadors)?
  Recommended: every member – the switch and the daily limits are the control.
- ~~Q3 What the invite unlocks~~ – answered: like a promo code, until the admin's *access ends*.
