<?php
// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * One toon's address book (#477, decisions/deck.md [contract] [typeahead] [address]): every thing of the toon's catalog a typeahead may pick, with its reference, its address where something can receive one, its kind, labels and where it comes from. The server writes it (GET /api/addresses/<toon>), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
 *
 * @phpstan-type AddressKind 'character'|'persona'|'look'|'location'|'item'|'chapter'|'group'|'variable'|'season'|'toon'|'product'
 * @phpstan-type OriginState 'published'|'draft'
 * @phpstan-type LocaleText array<string, string>
 * @phpstan-type Origin array{key: string, state: OriginState}
 * @phpstan-type AddressEntry array{ref: string, address?: string, kind: AddressKind, name: string, parent?: string, labels: LocaleText, origin: Origin, moods?: list<string>}
 * @phpstan-type AddressBook array{address_book: 2, toon: string, version: string, source: string, entries: list<AddressEntry>}
 */
final class AddressBookTypes
{
    /** the format's version (`address_book`) */
    public const FORMAT = 2;
    /** @var list<AddressKind> What an entry is – the one thing a typeahead filters by: an asker takes the kinds it may pick (a VN character event: character, persona, look; unlockLocation: location; an item event: item; a jump: chapter, group, season, toon, product; setVariable: variable). A jump's kinds are its target types (`JumpToTargetType`, a group is a chapter target with its label): `season:<number>` (the toon's season), `toon:<id>` (a toon: its page, and in the creator's jump field the way into its own book), `product:<id>` (a shop product of the toon or a shared one); XR is not listed: there is no catalog of XR targets. character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); location, item and chapter are referenced `<kind>:<id>`. Proposals the owner has not confirmed (#477): a chapter's group is referenced `chapter:<number>.<group>` (its `name` the label as an event stores it, the reference word a safe form of it) and a story variable `variable:<scope>.<name>` (scope = the owning toon or `global`; its `name` the variable as written in this toon). */
    public const ADDRESS_KIND = ['character', 'persona', 'look', 'location', 'item', 'chapter', 'group', 'variable', 'season', 'toon', 'product'];
    /** @var list<OriginState> `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's. */
    public const ORIGIN_STATE = ['published', 'draft'];
}
