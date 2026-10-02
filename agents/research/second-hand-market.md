# Second-hand market for SCUTOONS items – legal, tax and design research

Research date: 2026-10-02. Written for the owner (artist/publisher, Vienna, AT/EU) as input for the decision
"gifting only" vs. "closed-loop marketplace" vs. "real-money marketplace". Sources are linked inline; web pages
were accessed on 2026-10-02 unless a date is given.

> **Not legal or tax advice.** This is a structured research summary to prepare a conversation with a lawyer
> (gaming/fintech/IT law) and a Steuerberater. Several points below are explicitly flagged as **[LAWYER]** or
> **[TAX]** because the answer depends on details of the final design or on unsettled law.

---

## 0. TL;DR

- **Gifting (free) is low-risk.** No money, no VAT, no income tax, no payment regulation. What you need is abuse
  protection (account farming, scams, "gift me for PayPal" side deals), minors-safe rules, and clear terms.
- **The legal risk does not come from "a market" as such – it comes from *real money* and *cash-out*.** As soon
  as items can be turned into euros, (1) items get a market value, which is the classic trigger for **gambling law**
  if anything random is paid for; (2) holding or forwarding buyer money is a **payment service** (licence or a
  licensed payment provider needed); (3) sellers can owe **income tax**, and the platform has **DAC7 reporting**
  duties (for the physical twins) and possibly **VAT on the full price** for digital items (deemed-supplier rule).
- **A closed loop** (items can be swapped or bought with a platform currency that can only be earned/bought from
  SCUTOONS and never paid out) avoids most of this, but still needs consumer-law-compliant currency design and
  the limited-network rules if the currency is bought with money.
- **The physical twins are the natural candidate for real resale** (they are physical goods; the first-sale/
  exhaustion doctrine clearly applies to them). The cleanest model: the money and the object change hands
  outside SCUTOONS (or through a licensed payment partner), SCUTOONS only **records the ownership transfer**
  of the code, with a holding period, an optional voluntary/contractual creator royalty, and age 18+.
- How the industry does it: almost everyone big **forbids cash-out** (Steam, EVE, Fortnite, MTG Arena, Pokémon),
  or has a licensed money business behind it (Second Life/Tilia, Roblox DevEx with strict eligibility). Blizzard
  shut the Diablo III real-money auction house because it damaged the game itself.

---

## 1. Can readers legally resell digital items in the EU? (exhaustion, licence vs. ownership)

**Exhaustion doctrine.** Once a copy of a work is sold in the EU with the right holder's consent, the right
holder can no longer control its further distribution (first-sale doctrine).

- **UsedSoft v Oracle, CJEU C-128/11, 3 July 2012** – for *computer programs* (Software Directive 2009/24/EC)
  the distribution right is exhausted also for a downloaded copy, if the right holder grants a permanent licence
  for a fee; the first acquirer must make his own copy unusable.
  [EUR-Lex CELEX 62011CJ0128](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62011CJ0128)
- **Nederlands Uitgeversverbond v Tom Kabinet, CJEU C-263/18, 19 December 2019** – for other works (e-books;
  InfoSoc Directive 2001/29/EC) supplying a download for permanent use is *communication to the public*, not
  distribution, so **no exhaustion**: a second-hand market in digital copies needs the right holder's consent.
  [EUR-Lex CELEX 62018CJ0263](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62018CJ0263)
- **France, UFC-Que Choisir v Valve (Steam):** Tribunal de grande instance Paris (17 Sept 2019) said Steam games may
  be resold; **Cour d'appel de Paris reversed on 21 Oct 2022**, and the **Cour de cassation confirmed on
  23 Oct 2024** that Steam's no-resale clause stands.
  ([Dauphijuris on the 2022 appeal](https://www.dauphijuris.fr/arret-du-21-octobre-2022-de-la-cour-d-appel-de-paris---ufc-que-choisir-contre-valve--_ad187.html),
  [L'Informé on the Cour de cassation](https://www.linforme.com/tech-telecom/article/steam-vs-que-choisir-la-cour-de-cassation-interdit-la-revente-des-jeux-video-dematerialises_2179.html))

**What this means for SCUTOONS**

- Purely digital SCUTOONS items (story finds, camera scans, crafted items, artwork/audio inside the app) are
  digital content/licences, not copies the reader owns. Under Tom Kabinet the reader has **no statutory right to
  resell** them; **you decide** in the terms whether they are transferable, giftable or sellable, and on what terms
  (fees, holding periods, non-transferable flags). This is the same position Steam, Epic, Blizzard etc. rely on.
- Terms still have to pass **unfair-terms control** (Directive 93/13/EEC; in AT § 879(3) ABGB and § 6 KSchG). Typical
  risk points: deleting items/balances without reason, one-sided changes, forfeiting paid value on account
  closure. Make "items are a licence, transfer only via the official feature" clear and fair. **[LAWYER]**
- **Physical twins are different:** the record/art piece is a physical good, sold by you → exhausted, the buyer may
  always resell the object. You cannot legally stop the object being sold; you *can* decide whether the
  **platform account binding** (the QR code's owner, the virtual item, the certificate's register entry) follows it,
  and on which conditions (e.g. both parties confirm, holding period). Keep the terms honest: "the object is
  yours; the digital twin moves with it through the official transfer".

---

## 2. Taxes

### 2.1 VAT

| Case | VAT result |
|---|---|
| SCUTOONS sells items or currency to readers | Normal supply by SCUTOONS. Electronically supplied service → VAT of the customer's country; EU-wide B2C e-services above €10,000/year go through **OSS**. AT small-business exemption (*Kleinunternehmer*) is **€55,000 gross** turnover since 2025 ([USP.gv.at](https://www.usp.gv.at/aktuelles/newsliste/kleinunternehmerregelung-ab-2025.html)). |
| Reader gifts an item | No consideration → no VAT. |
| Private reader sells a **digital item** to another reader *through the SCUTOONS marketplace* | **Art. 9a Implementing Regulation 282/2011:** a platform that intermediates electronically supplied services is *presumed to act in its own name* and becomes the VAT supplier to the buyer. CJEU **Fenix International (OnlyFans), C-695/20, 28 Feb 2023** upheld this – the platform accounts for VAT on the **full price**, not just its fee ([EUR-Lex 62020CJ0695](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=celex%3A62020CJ0695), [DLA Piper](https://www.dlapiper.com/en-us/insights/publications/2023/03/validity-of-article-9a-of-council-implementing-regulation-282-2011-confirmed)). Whether the presumption bites when the underlying seller is a private non-taxable person is debated – **[TAX]**, but plan for "VAT on the gross sale price" as a realistic case. This is the single biggest economic surprise of a real-money digital marketplace. |
| Private reader sells a **physical twin** | Private sale of goods → no VAT. If the reader acts commercially (regular, profit-oriented) he becomes an entrepreneur (but usually a Kleinunternehmer). The platform's own fee is a normal service by SCUTOONS (VAT on the fee). The ViDA package (Council Directive (EU) 2025/516) extends deemed-supplier rules to accommodation/transport from 2030, not to second-hand goods between consumers. |
| Platform currency bought with money | VAT when sold (if it can only be spent on SCUTOONS's own content, treated like a prepayment/voucher – single-purpose vs. multi-purpose voucher rules, Art. 30a-30b VAT Directive) **[TAX]**. |

### 2.2 Income tax for selling readers (Austria; Germany for comparison)

- **AT – Spekulationsgeschäfte, § 31 EStG 1988:** gains from selling private assets (other than securities/crypto
  under § 27 and real estate under § 30) are taxable if bought and sold **within one year**. The gains are tax-free
  if all such gains of the year total **≤ €440 (Freigrenze** – exceed it by €1 and everything is taxable). Held
  longer than a year → tax-free. Losses are not deductible against other income.
  ([RIS § 31 EStG](https://ris.bka.gv.at/eli/bgbl/1988/400/P31/NOR40137548), [JUSLINE](https://www.jusline.at/gesetz/estg/paragraf/31))
  NFTs and similar digital assets are treated as "other assets" under § 31 (not as cryptocurrency) per BMF practice
  ([BMF Findok info on § 31 and token swaps](https://findok.bmf.gv.at/findok/iwg/143/143480/143480.1.pdf)); a SCUTOONS item
  would most likely be treated the same. Items received **for free** (found, gifted) have acquisition cost 0, so
  the whole price can be a gain – but only if sold within a year of acquisition.
- **Commercial activity:** a reader who systematically buys and resells (flipping) can become *gewerblich* →
  business income, possibly VAT registration (normally Kleinunternehmer). On a marketplace he must then be
  labelled a trader (see § 5).
- **DE (many readers):** § 23 EStG private Veräußerungsgeschäfte, one-year period, **Freigrenze €1,000** per year
  since 2024 (previously €600). **[TAX]** for cross-border sellers.
- **Practical consequence for SCUTOONS:** you are not liable for the readers' income tax, but you should (a) say in
  the terms that sellers are responsible for their taxes, (b) give each seller a yearly sales statement, and
  (c) a **holding period ≥ 1 year** is *not* something you need – it is just a fact that long-held items are
  tax-free for AT private sellers.

### 2.3 DAC7 platform reporting (Directive (EU) 2021/514; AT: **DPMG**, in force 1 Jan 2023)

- Applies to "platform operators" that facilitate **relevant activities**: rental of immovable property, personal
  services, **sale of goods**, rental of means of transport. "Goods" = **tangible property**. A sale of a *purely
  digital* item is therefore **not** reportable under DAC7 (unless it is framed as a personal service). The sale of a
  **physical twin** facilitated through the platform **is**.
  ([Directive 2021/514 on EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32021L0514),
  [DPMG on RIS](https://www.ris.bka.gv.at/eli/bgbl/i/2022/108/P0/NOR40246155),
  [BMF overview PDF, 19 Mar 2024](https://www.bmf.gv.at/dam/jcr:756a6799-d2ba-4488-b7f7-6cfd09121fa9/BMF-Allgemeines_DPMG%20-%20Stand%2019.03.2024.pdf))
- **Excluded sellers:** a seller of goods is not reported if he made **fewer than 30 sales AND received ≤ €2,000**
  in the calendar year. Reaching either 30 sales or more than €2,000 makes him reportable. (Some secondary summaries
  phrase this the other way round – the exclusion needs *both* conditions.)
- **What the operator must do:** collect and verify seller data (name, address, date of birth, TIN, VAT number if
  any, bank account/IBAN), record consideration and fees **per quarter** and the number of sales; report to the AT
  tax office (via FinanzOnline/Plattformportal) by **31 January** of the following year and give the seller a copy.
  Non-compliance is a fiscal offence (fines) and sellers who don't provide data must eventually be blocked.
- **Open point [TAX]:** if SCUTOONS only records the ownership change and the price is paid *outside* the platform,
  is SCUTOONS still "facilitating a sale for consideration"? DAC7 covers consideration that is "known or reasonably
  knowable". If SCUTOONS never sees a price, there is a good argument it is not reportable – but check before launch,
  and decide early whether the platform shows/handles prices for physical twins.

---

## 3. Financial regulation

### 3.1 When does a balance become e-money or a payment service?

- **E-Money Directive 2009/110/EC (AT: E-Geldgesetz 2010):** electronically stored monetary value, issued on
  receipt of funds, used for payments, **accepted by persons other than the issuer** = e-money → licence (FMA).
  Pure in-game currency that only buys the issuer's own content is generally not e-money.
- **PSD2 (Directive (EU) 2015/2366; AT: ZaDiG 2018):** if SCUTOONS **receives the buyer's money and forwards it to
  the seller**, that is a payment service (money remittance / acquiring) → needs authorisation, *unless* an exclusion
  applies. The "commercial agent" exclusion (Art. 3(b)) is narrow (agent for only one side) and marketplaces usually
  cannot use it. Standard solution: use a licensed **platform payment provider** (Stripe Connect, Adyen for
  Platforms, Mangopay, etc.) so the money never sits on SCUTOONS's account.
- **Limited-network exclusion, PSD2 Art. 3(k)** (also used by EMD): instruments usable only in a limited network or
  for a very limited range of goods/services are excluded; above **€1 million** of transactions in 12 months the
  issuer must **notify** the regulator (Art. 37(2)). EBA guidance:
  [EBA/GL/2022/02, 24 Feb 2022](https://www.eba.europa.eu/sites/default/files/document_library/Publications/Guidelines/2022/EBA-GL-2022-02%20GL%20on%20limited%20network%20exclusions/1027516/Final%20report%20on%20draft%20Guidelines%20on%20the%20limited%20network%20exclusion%20under%20PSD2.pdf).
  A SCUTOONS currency spendable only on SCUTOONS content is the textbook case; once **other readers can be paid
  with it** and can **cash it out**, the exclusion is in doubt.
- **PSD3 / Payment Services Regulation (PSR):** provisional political agreement **27 Nov 2025**; formal adoption
  expected 2026, application later. The limited-network exclusion is **narrowed** to a "single limited network" and
  the notification threshold calculation harmonised
  ([Taylor Wessing, Nov 2025](https://www.taylorwessing.com/en/insights-and-events/insights/2025/11/eu-lawmakers-strike-a-deal-on-payments-reforms),
  [Norton Rose Fulbright](https://www.nortonrosefulbright.com/en/knowledge/publications/cedd39c6/psd3-and-psr-from-provisional-agreement-to-2026-readiness)).
- **Rule of thumb:** *closed loop* (money in → currency → SCUTOONS content; no reader-to-reader money; no cash out)
  = outside payment regulation. *Reader-to-reader money* = payment service → partner with a licensed PSP. *Cash out of
  a platform balance* = e-money/payment-institution territory → licensed partner mandatory. **[LAWYER]**

### 3.2 Anti-money-laundering

- Tradeable items with real-money value are a known laundering and fraud channel (stolen cards → buy items →
  sell to clean buyers; CS skins are the classic example). The licensed PSP does KYC on sellers, but SCUTOONS should
  still have fraud controls (holding periods, velocity limits, chargeback handling).
- **Art dealers** are obliged entities under AMLD (5th AMLD, Directive (EU) 2018/843) for art transactions of
  **≥ €10,000** (in AT implemented through the GewO for traders/art dealers; FM-GwG for financial institutions). The new
  **AML Regulation (EU) 2024/1624** applies from **10 July 2027** and keeps art traders/intermediaries in scope
  ([Binder Grösswang, 2024](https://www.bindergroesswang.at/en/insights/law-blog/2024/the-eu-money-laundering-package)).
  A price cap well below €10,000 per physical-twin sale keeps SCUTOONS clearly out of this. **[LAWYER]**

### 3.3 MiCA (only relevant if items became blockchain tokens/NFTs)

- Regulation (EU) 2023/1114 excludes crypto-assets that are **unique and non-fungible** (Art. 2(3)); NFTs issued in
  large series or fractionalised may still be in scope (recitals 10-11; ESMA guidance). SCUTOONS items in a central
  database are **not** crypto-assets at all – another reason not to "tokenise" them. Austria's NFT tax practice
  (§ 31 EStG) would apply anyway.

---

## 4. Gambling / loot boxes: why tradability + real-money value is the trigger

Gambling under most EU laws needs **stake + chance + prize of monetary value**. Paying for a random pack
satisfies stake and chance; whether the prize has *value* often depends on whether it can be sold for money.

- **Belgium:** Gaming Commission report (25 Apr 2018) found loot boxes in Overwatch, FIFA 18 and CS:GO to be illegal
  gambling; Valve disabled CS:GO case opening in BE and NL
  ([PC Gamer](https://www.pcgamer.com/belgiums-gambling-commission-rules-against-loot-boxes-in-overwatch-fifa-18-and-csgo/),
  [Steam news](https://steamcommunity.com/news/post/2838841185434278558)).
- **Netherlands:** Kansspelautoriteit study (Apr 2018) found 4 of 10 loot boxes illegal – precisely those whose
  contents were **transferable and therefore had market value**
  ([KSA study PDF](https://kansspelautoriteit.nl/publish/library/17/study_into_loot_boxes_-_a_treasure_or_a_burden_-_eng.pdf)).
  Raad van State (9 Mar 2022) annulled the €10m order against EA: FIFA packs are part of a skill game, not a
  stand-alone game of chance ([Raad van State](https://www.raadvanstate.nl/@130206/dwangsom-onterecht-opgelegd-loot-boxes/)).
- **Austria:** BG Hermagor (26 Feb 2023) held FUT packs to be illegal gambling and ordered Sony to refund €338.26
  ([heise](https://www.heise.de/news/Lootboxen-in-FIFA-Sony-soll-in-Oesterreich-Zahlungen-zurueckerstatten-7536591.html),
  [help.ORF.at](https://help.orf.at/stories/3218379/)); other lower courts went both ways
  ([Schönherr](https://www.schoenherr.eu/content/lootboxen-und-weitere-entwicklungen-im-glucksspiel-entertainment)).
  The **OGH (6 Ob 228/24h, 18 Dec 2025)** dismissed a €20,000 claim: packs must be judged with the game as a whole,
  game success does not depend predominantly on chance, and – central for us – the virtual content **stays in the game
  and cannot be exchanged back for money**
  ([gamers.at](https://www.gamers.at/der-ogh-trennt-das-fifa-pack-vom-spielautomaten),
  [kicker](https://www.kicker.de/grosses-fifa-urteil-lootboxen-sind-in-oesterreich-kein-gluecksspiel-1188291/artikel)).
  The OGH left other designs open.
- **Coming EU rules:** the CPC Network's *Key Principles on in-game virtual currencies* (21 Mar 2025) – prices must be
  shown in euros, no hiding of costs, withdrawal rights, special care for minors
  ([Commission PDF](https://commission.europa.eu/document/download/8af13e88-6540-436c-b137-9853e7fe866a_en?filename=Key+principles+on+in-game+virtual+currencies.pdf)).
  The **Digital Fairness Act** is announced for Q4 2026 and not yet tabled (as of this date); loot boxes for minors,
  virtual-currency transparency and dark patterns are expected targets
  ([EP legislative train](https://www.europarl.europa.eu/legislative-train/theme-protecting-our-democracy-upholding-our-values/file-digital-fairness-act)).

**What this means for SCUTOONS:** today items are found, scanned or crafted – **no stake**, so no gambling issue, even
with random drops. The danger zone would be the *combination* of (a) anything random that costs money (paid packs,
paid rerolls, paid "mystery" camera charges) **and** (b) resale for real money. If a real-money market comes, keep
every paid mechanic **deterministic** (you know what you buy) or keep randomly obtained items **non-sellable**. Also
watch third-party "skin casino" sites: Valve's CS skins were used as casino chips by outside sites (Valve sent cease
and desist letters to 40+ sites in 2016 after the Washington State Gambling Commission intervened –
[GeekWire](https://www.geekwire.com/2016/valve-fires-back-warning-skins-gambling/)). Terms should ban commercial use and
wagering of items.

---

## 5. Consumer protection and marketplace-operator duties

- **Digital Services Act (Regulation (EU) 2022/2065):** SCUTOONS is already a hosting service/online platform
  (user content, profiles). A marketplace adds the "online marketplace" duties: **trader traceability (Art. 30)**,
  compliance by design (Art. 31), information to buyers about illegal products (Art. 32). **Micro and small
  enterprises (< 50 staff and ≤ €10m turnover) are exempt from Section 3 and 4** (Arts. 19 and 29) – including Arts. 28,
  30-32 – as long as they are not a VLOP. Still applicable: terms (Art. 14), notice-and-action (Art. 16), statements
  of reasons (Art. 17), single point of contact (Arts. 11-12). [EUR-Lex DSA](https://eur-lex.europa.eu/eli/reg/2022/2065/oj)
- **Omnibus Directive (EU) 2019/2161** (AT: VRUG/FAGG changes 2022): an online marketplace must tell buyers
  **whether the seller is a trader or a consumer** (and that consumer-protection rights don't apply in C2C), and how
  listings are ranked. Applies regardless of size.
- **Consumer Rights Directive:** for SCUTOONS's own sales of digital content, the 14-day withdrawal right lapses
  only with the consumer's express consent and acknowledgement at purchase. In **C2C** sales between readers there is
  no withdrawal right and no statutory warranty from SCUTOONS – say so clearly.
- **P2B Regulation (EU) 2019/1150** applies only toward *business* users (traders selling on the platform): terms
  transparency, ranking parameters, complaint handling, 15 days notice for term changes. Not relevant if only
  consumers sell; relevant once galleries/shops or the "embassy" partners sell through SCUTOONS.
- **Secondary-ticketing-style rules:** the Omnibus ban on reselling tickets bought with bots (Annex I no. 23a UCPD)
  is ticket-specific; there is no general EU price-cap rule for collectibles. Price caps are a design choice.
- **Minors (Austria):** under § 170(3) ABGB minors can validly conclude only everyday transactions of minor value
  appropriate for their age; anything else needs parental consent. Digital-consent age for data processing is **14**
  in Austria (§ 4(4) DSG). Payment providers require sellers to be 18 for KYC anyway. → **Selling: 18+ only**;
  gifting: fine for minors, with limits.

---

## 6. Art-specific: artist's resale right and royalties

- **Folgerecht – Directive 2001/84/EC; AT § 16b UrhG** ([JUSLINE](https://www.jusline.at/gesetz/urhg/paragraf/16b),
  [Bildrecht](https://bildrecht.at/bildnutzerinnen/folgerecht/)):
  - Applies to the resale of **originals of works of fine art** – including limited copies made by or under the
    artist's authority that are **numbered or signed** – after the first transfer by the artist.
  - Only if an **art-market professional** (auction house, gallery, art dealer) is involved as seller, buyer or
    intermediary, and the price is **≥ €2,500**.
  - Rate: 4% of the first €50,000, 3% of the next €150,000, 1% of the next €150,000, 0.5% of the next €150,000,
    0.25% beyond; cap **€12,500**. Inalienable, cannot be waived in advance; claimed via the collecting society
    (Bildrecht in AT). The art professional is liable as guarantor/payer.
  - **For SCUTOONS:** a vinyl record or a hoodie is not a work of fine art. A signed/numbered art print or original
    artwork could be. Sales between readers below €2,500 are out of scope anyway. If SCUTOONS itself acts as the
    paid intermediary for a ≥ €2,500 resale of a signed original, SCUTOONS could count as the art professional and
    would owe/collect the Folgerecht for the artist (the owner himself, via Bildrecht). Edge case – **[LAWYER]** only
    if high-priced originals are planned.
- **Platform royalties on resales:** there is no law that *gives* the creator a royalty on digital-item resales, but
  nothing stops the platform from **charging a fee that goes to the creator** on every sale *through its own market* –
  that is just a contractual marketplace fee (Steam's "game-specific fee" is exactly this, see § 7). It is enforceable
  because the transfer can only happen on the platform. It is **not** enforceable for the physical object sold
  elsewhere – only for the platform transfer of the digital twin.
- **NFT lesson:** on-chain "royalties" were only ever marketplace conventions. **OpenSea made creator fees optional
  from 31 Aug 2023** and sunset its Operator Filter (enforcement kept until 29 Feb 2024 for some collections) because
  other marketplaces didn't enforce them
  ([The Block, 17 Aug 2023](https://www.theblock.co/news/web3/2023-08-17-opensea-disables-royalty-enforcement-tool-makes-creator-fees-optional-246095),
  [Decrypt](https://decrypt.co/152878/opensea-make-creator-royalties-optional-nft-trades)). Royalties hold only where
  one party controls the transfer – a central SCUTOONS registry is actually *better* for royalties than a blockchain.
  Reddit's NFT avatars show the other risk: the program and the in-app Vault were wound down (Vault export deadline
  1 Jan 2026), on-platform transfers removed
  ([Crypto Briefing](https://cryptobriefing.com/reddit-collectible-avatar-program-sunset/)).

---

## 7. How other games and platforms handle it

| Platform | Model | Key facts / limits |
|---|---|---|
| **Steam Community Market** (Valve) | Real money in, **no cash out** | Wallet funds "have no cash value", are not a property right and can't be withdrawn ([Steam Subscriber Agreement](https://store.steampowered.com/subscriber_agreement/)). Seller fee: 5% Steam fee + game-specific fee (10% for CS2/Dota 2) ≈ 15% total. Wallet cap: balance plus 24 h spend ≤ US$2,000. Trade holds (up to 15 days without Steam Guard Mobile Authenticator). Since **July 2025** CS2 **Trade Protection**: traded items can be reversed by the sender for 7 days and can't be re-traded/listed meanwhile ([swap.gg explainer](https://swap.gg/blog/cs2-7-day-trade-lock-explained)). The cash-out happens on grey third-party sites that Valve doesn't control – the root of the CS gambling scandal. |
| **Diablo III RMAH** (Blizzard) | Real-money auction house with cash out (via PayPal, 15% fee) | Closed **18 Mar 2014**: it "ultimately undermines Diablo's core gameplay: kill monsters to get cool loot" (John Hight) ([PCGamesN](https://www.pcgamesn.com/diablo/diablo-3-auction-house-scrapped-itll-be-shut-down-march), [NBC News](https://www.nbcnews.com/technolog/no-money-no-problems-blizzard-axes-diablo-3-auction-house-4B11192195)). Lesson: a cash market changes *why* people play and turns drops into income. |
| **EVE Online** (CCP) | PLEX bought with money, tradeable for in-game ISK; **RMT forbidden** | Players can buy game time from each other with ISK; selling ISK/items/characters for real money is banned, assets removed, accounts banned ([EVE RMT policy](https://support.eveonline.com/hc/en-us/articles/7370633552284-Real-Money-Trading-RMT)). Closed loop with an official one-way bridge. |
| **Second Life** (Linden Lab) | Linden dollars, **cash out allowed** | Cash out is handled by **Tilia Inc.**, a Linden subsidiary that is a registered money services business and licensed US money transmitter, with ID verification ([Second Life news](https://community.secondlife.com/news/tools-and-technology/tilia-officially-begins-operations-today-in-second-life-r39/)). Shows the cost of real cash-out: a regulated money business. |
| **Roblox DevEx** | Robux closed loop, **cash out only for creators** | Earned (not bought) Robux, minimum 30,000, age/ID/tax checks; rate US$0.0038 per Robux since 5 Sept 2025 ([Roblox help](https://en.help.roblox.com/hc/en-us/articles/13061189551124-Developer-Exchange-Help-and-Information-Page)). Players can't cash out; Limited items trade for Robux only. |
| **Fortnite** (Epic) | No trading; gifting from the shop only | Gifting needs 2FA, ≥ 48 h friendship, account level; limited gifts per day; V-Bucks not transferable ([summary](https://slaykeys.com/en/guides/how-to-gift-in-fortnite)). |
| **Magic: The Gathering Arena** | No trading at all | Cards are bound to the account; wildcards/crafting instead of a market. |
| **Pokémon TCG Live** | Trading **removed** (TCG Online had it) | Replaced by crafting with earned crystals; TCG Online shut down 5 Jun 2023 ([Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Trading_Card_Game_Live)). |
| **Pokémon TCG Pocket** | Restricted friend trading (since Jan 2025) | Only same-rarity, rare cards cost Trade Tokens, Trade Stamina recovers slowly ([VGC](https://www.videogameschronicle.com/guide/pokemon-tcg-pocket-trading-how-to-get-trade-tokens/)). |
| **Pokémon GO** | Free trading, heavy friction | Must be friends and within ~100 m (except remote trade at top friendship), Stardust costs, "special trades" (legendary/shiny/new) once a day, costs fall with friendship level ([Pokémon GO Hub](https://pokemongohub.net/post/guide/pokemon-go-trading-ultimate-go-hub-guide/)). Trading is social, not commercial. |
| **Reddit Collectible Avatars** | NFTs on Polygon, resale on OpenSea | Wound down 2025: shop closed, Vault retired (export by 1 Jan 2026), on-platform transfers removed (see § 6). |
| **Nintendo / Sony / Xbox** | Digital purchases non-transferable | Licence to the account; no resale; gifting at most of store purchases. |

**Anti-speculation tools seen in the wild:** non-tradeable flags (account-bound items), trade/market holds (Steam
7-15 days), reversal windows (CS2 7 days), price ceilings (Steam's US$2,000 cap), sales fees (Steam ~15%), daily trade
limits (Pokémon GO special trade once a day, TCG Pocket stamina), friendship duration/level requirements (Fortnite
48 h, Pokémon GO), physical proximity (Pokémon GO), account age and 2FA (Fortnite), rarity-based trade costs (TCG
Pocket), creator-only cash-out (Roblox).

---

## 8. Recommendations for SCUTOONS, staged

### Stage (a) – Gifting only (free) – *recommended now*

No VAT, no income tax, no DAC7, no payment regulation, no gambling issue. What's needed:

1. **Terms:** items are a licence bound to the account; transfer only via the official gift feature; gifting is free;
   **any sale or exchange for money outside SCUTOONS is prohibited** and can lead to removal of the item (this is
   the Steam/EVE/Fortnite clause – it keeps you out of the "you facilitate a market" discussion). **[LAWYER]** for
   wording.
2. **Abuse limits** (pick from):
   - only between accounts that have existed ≥ 7 days and are verified (email; 2FA later),
   - gifts per account per day/week limited (e.g. 5/day), received gifts per day limited,
   - recipient must accept; sender sees a clear "this is permanent" confirmation,
   - gifted items get a **re-gift cooldown** (e.g. 7-30 days) so items can't be funnelled through chains to a buyer,
   - the existing 99-per-item cap applies to the receiver,
   - log every transfer (sender, receiver, time) for support and fraud cases.
3. **Non-giftable by default:** starter items, story-progress keys, items earned by personal actions where the point is
   that *you* did it (camera scans, daily-limited finds – otherwise one reader farms for many), membership items.
   Mark giftable items explicitly (`transferable: true`) rather than the other way round.
4. **Physical twins:** treat the transfer of a code's ownership as its own flow: both accounts confirm; the old owner
   loses the virtual item; the certificate stays valid (the signed certificate proves the object, the server register
   says who owns it now); show provenance ("2nd owner since …"). This is how a gift *and* a private sale of the object
   (money paid outside SCUTOONS) both end up in the same, clean record – the platform only records the ownership change.
5. **Minors:** gifting is fine; consider gifting only between "friends" (if you add a friend system), no public gift
   requests, and a report button. No money anywhere in the flow.

### Stage (b) – Closed-loop marketplace (no cash out)

Readers swap or "sell" items for a SCUTOONS currency that can only be spent on SCUTOONS content and **never paid out**.

- If the currency is **only earned** (not bought): basically the same risk profile as gifting. Best option.
- If the currency is also **sold for money**: you are selling digital content/vouchers → VAT on sale, CPC key principles
  (show euro prices, no forced bundles, withdrawal-right handling, minors), limited-network exclusion (fine while
  usable only for SCUTOONS content; notify the FMA if > €1m/year; re-check under PSR). Avoid paid random mechanics.
- Keep it closed for real: no "convert to voucher for the shop" that could be resold, no transfer of currency between
  readers (only items for currency), Steam-like limits (holds, fee/sink, price ceiling per item) to stop speculation.
- Be aware the grey market moves outside (people pay each other by PayPal and transfer in-game). Terms + detection
  (unusual one-sided transfers, new accounts) + willingness to reverse.

### Stage (c) – Real-money marketplace

Only worth it for the **physical twins** (where resale is legally a right anyway and the value is real). For purely
digital items the downsides (VAT on gross price per Fenix, payment regulation, gambling trigger for any paid randomness,
speculation, the Diablo III effect) outweigh the benefit. If done:

1. **Payments:** a licensed platform PSP (split payments, seller KYC, payouts). SCUTOONS never holds buyer money.
   No platform balance with cash-out. **[LAWYER]** to confirm no own licence is needed (ZaDiG/E-GeldG, FMA).
2. **Sellers:** 18+, KYC via PSP, trader/consumer self-declaration shown to buyers (Omnibus), DSA trader traceability if
   you are no longer a micro/small company.
3. **Tax:** DAC7 data collection and yearly report for physical-twin sellers (≥ 30 sales or > €2,000); yearly statement
   for every seller; VAT on SCUTOONS's own fee; clarify Art. 9a deemed-supplier VAT if digital items are ever sold for
   money. **[TAX]**
4. **Fees and royalties:** a transparent fee (e.g. 5-10%) of which a share goes to the creator – contractually enforceable
   because the ownership register only moves through the platform. Folgerecht only for signed originals ≥ €2,500 sold
   through a professional – unlikely for current editions. **[LAWYER]** if originals are sold.
5. **Speculation limits:** holding period after acquisition (e.g. 30-90 days), price ceiling per edition (e.g. a
   multiple of the original price, keeps it under any €10,000 AML line), no sales during the first weeks after a
   release, limit of listings per account, sale reversal window for fraud (CS2-style 7 days), public price history.
6. **Keep non-tradable:** everything obtained by chance (camera scans with limits, random finds), story keys, membership,
   unique "only once" items bound to a person, anything given to minors, crafting intermediates. Never combine paid
   randomness with resale.
7. **Physical twin flow:** buyer pays via PSP → seller ships the object (tracked) → buyer confirms receipt (or deadline)
   → payout + ownership change in the register in one step; dispute process. Alternative with far less regulation:
   **no price on the platform at all** – sellers sell wherever they like (eBay, willhaben, in person), SCUTOONS offers
   only the verified ownership transfer (both confirm; maybe a small flat transfer fee paid by the buyer as a SCUTOONS
   service). This keeps SCUTOONS out of payments and probably out of DAC7, and is the recommended first step for
   physical twins. **[TAX]** confirm the DAC7 point.

### What needs a professional

| Topic | Who |
|---|---|
| Terms of use for transferable items, no-resale clause, unfair-terms check, item removal | Lawyer (IT/consumer law) |
| Whether any planned flow is a payment service or e-money (ZaDiG 2018, E-GeldG, FMA notification) | Lawyer (fintech) |
| Gambling assessment of any paid random mechanic combined with tradability (GSpG) | Lawyer (gaming) |
| VAT treatment: Art. 9a deemed supplier, currency as voucher, OSS | Steuerberater |
| DAC7/DPMG: is an ownership-transfer-only service a reportable platform? | Steuerberater |
| Folgerecht for signed originals; Bildrecht membership | Lawyer / Bildrecht |
| DSA size exemption, trader traceability, minors | Lawyer |

---

## Sources (selection, primary sources first)

- CJEU C-128/11 UsedSoft (3 Jul 2012) – https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62011CJ0128
- CJEU C-263/18 Tom Kabinet (19 Dec 2019) – https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:62018CJ0263
- CJEU C-695/20 Fenix International (28 Feb 2023) – https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=celex%3A62020CJ0695
- Directive (EU) 2021/514 (DAC7) – https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX:32021L0514
- DPMG (AT, BGBl. I 108/2022) – https://www.ris.bka.gv.at/eli/bgbl/i/2022/108/P0/NOR40246155
- BMF DPMG overview (19 Mar 2024) – https://www.bmf.gv.at/dam/jcr:756a6799-d2ba-4488-b7f7-6cfd09121fa9/BMF-Allgemeines_DPMG%20-%20Stand%2019.03.2024.pdf
- § 31 EStG 1988 – https://ris.bka.gv.at/eli/bgbl/1988/400/P31/NOR40137548
- Kleinunternehmer 2025 (USP) – https://www.usp.gv.at/aktuelles/newsliste/kleinunternehmerregelung-ab-2025.html
- EBA/GL/2022/02 limited network exclusion – https://www.eba.europa.eu/publications-and-media/press-releases/eba-publishes-final-guidelines-limited-network-exclusion
- PSD3/PSR deal (Nov 2025) – https://www.taylorwessing.com/en/insights-and-events/insights/2025/11/eu-lawmakers-strike-a-deal-on-payments-reforms
- DSA – https://eur-lex.europa.eu/eli/reg/2022/2065/oj
- CPC Key Principles on in-game virtual currencies (21 Mar 2025) – https://commission.europa.eu/document/download/8af13e88-6540-436c-b137-9853e7fe866a_en?filename=Key+principles+on+in-game+virtual+currencies.pdf
- Digital Fairness Act status – https://www.europarl.europa.eu/legislative-train/theme-protecting-our-democracy-upholding-our-values/file-digital-fairness-act
- KSA loot box study (2018) – https://kansspelautoriteit.nl/publish/library/17/study_into_loot_boxes_-_a_treasure_or_a_burden_-_eng.pdf
- Raad van State FIFA (9 Mar 2022) – https://www.raadvanstate.nl/@130206/dwangsom-onterecht-opgelegd-loot-boxes/
- OGH 6 Ob 228/24h (18 Dec 2025), report – https://www.gamers.at/der-ogh-trennt-das-fifa-pack-vom-spielautomaten
- BG Hermagor (26 Feb 2023), report – https://help.orf.at/stories/3218379/
- § 16b UrhG Folgerecht – https://www.jusline.at/gesetz/urhg/paragraf/16b ; Bildrecht – https://bildrecht.at/bildnutzerinnen/folgerecht/
- Steam Subscriber Agreement – https://store.steampowered.com/subscriber_agreement/
- EVE Online RMT policy – https://support.eveonline.com/hc/en-us/articles/7370633552284-Real-Money-Trading-RMT
- Roblox DevEx – https://en.help.roblox.com/hc/en-us/articles/13061189551124-Developer-Exchange-Help-and-Information-Page
- Second Life / Tilia – https://community.secondlife.com/news/tools-and-technology/tilia-officially-begins-operations-today-in-second-life-r39/
- OpenSea creator fees (17 Aug 2023) – https://www.theblock.co/news/web3/2023-08-17-opensea-disables-royalty-enforcement-tool-makes-creator-fees-optional-246095
- UFC-Que Choisir v Valve, Cour de cassation (23 Oct 2024), report – https://www.linforme.com/tech-telecom/article/steam-vs-que-choisir-la-cour-de-cassation-interdit-la-revente-des-jeux-video-dematerialises_2179.html

Secondary sources (news, law-firm notes, fan wikis) are used only for game facts and for case reports where the
judgment itself was not fetched; verify against the primary text before relying on a detail.
