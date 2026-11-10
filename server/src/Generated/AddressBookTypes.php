<?php
// GENERATED from server/schema/api/address-book.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * One toon's address book (#477, decisions/deck.md [contract] [typeahead]): every addressable thing of the toon's catalog with its address, kind, labels and where it comes from. The server writes it (GET /api/toons/<toon>/address-book), the creator's VN events and the deck editor filter it in one typeahead each; whether an address is valid stays the core's on save (/validate). Edit this file, run `npm run generate` (server/schema/generate.ts writes the Rust, TS and PHP types), commit the outputs.
 *
 * @phpstan-type AddressKind 'character'|'persona'|'look'|'location'|'item'|'chapter'
 * @phpstan-type OriginState 'published'|'draft'
 * @phpstan-type LocaleText array<string, string>
 * @phpstan-type Origin array{key: string, state: OriginState}
 * @phpstan-type AddressEntry array{address: string, kind: AddressKind, name: string, parent?: string, labels: LocaleText, origin: Origin, moods?: list<string>}
 * @phpstan-type AddressBook array{address_book: 1, toon: string, version: string, source: string, entries: list<AddressEntry>}
 */
final class AddressBookTypes
{
    /** the format's version (`address_book`) */
    public const FORMAT = 1;
    /** @var list<AddressKind> What an entry is – the one thing a typeahead filters by: an asker takes the kinds it can address (a VN character event: character, persona, look; unlockLocation: location; an item event: item; a jump: chapter). character / persona / look are the levels of address.rs's `Target` (`character:<id>[.<persona>[.<look>]]`); the others are a place `<kind>:<id>`. */
    public const ADDRESS_KIND = ['character', 'persona', 'look', 'location', 'item', 'chapter'];
    /** @var list<OriginState> `published`: the newest version of the origin document is the published one; `draft`: a newer draft exists (or nothing is published yet), and the entry is the draft's. */
    public const ORIGIN_STATE = ['published', 'draft'];
}
