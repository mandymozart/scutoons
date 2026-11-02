<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;

/**
 * The key of a stored entity (agents/DATA.md §2 "Entity keys"): `<kind>[:<toon>][:<number>][:<locale>]`.
 *
 *   toon:<toon>                          the toon's index entry (list entry, status, languages, names, the toon settings – texts per language)
 *   toon-settings:<toon>:<l>             legacy: the toon's per-language settings (title, description, release, status) – since agents/NEUTRAL.md step 4 they
 *                                        are locale maps in toon:<toon> (with the language-neutral ones: map, tile …); read while the entry has no texts
 *   season:<toon>:<n>                    chapter:<toon>:<n>
 *                                        one language-neutral document each, texts as locale maps (NeutralDoc, agents/NEUTRAL.md)
 *   season:<toon>:<n>:<l>  chapter:<toon>:<n>:<l>
 *                                        the legacy per-locale documents: read while the entity has no neutral document
 *   locations:<toon>:<l>  routes:<toon>:<l>                            the map
 *   cast:<toon>  credits:<toon>                                        language-neutral, per toon
 *   season-script:<toon>:<n>:<l>  chapter-script:<toon>:<n>:<l>
 *                                        a season's / a chapter's screenplay per locale (creator material, CREATOR_ONLY_KINDS;
 *                                        research/KANON.md §3 – the toon's `script:<toon>:<l>` became season 1's, 2026-10-21-season-scripts)
 *   kanon:<subject>:<l>                  a Kanon text per locale (creator material; subject `toon.<toon>`,
 *                                        `character.<id>[.<persona>[.<look>]]`, `place.<id>` – KANON_SUBJECT_PATTERN; `id` = the
 *                                        subject, `toon` set for a toon's text; research/KANON.md)
 *   storyboard:<toon>:<n>                a chapter's storyboard: `{ storyboard: 2, toon, chapter, scenes }`, language-neutral
 *                                        (creator material, CREATOR_ONLY_KINDS; agents/research/STUDIO-IN-CREATOR.md §2.2, #120)
 *   characters  moods  asset-requests  people  roles  events  map  colorations
 *                                        platform-wide, language-neutral (`colorations` = the custom collage presets –
 *                                        CollagePresets, agents/research/COLLAGE.md §12)
 *   place:<id>  route:<id>                                         one document per place / route of the world map (ID_KINDS)
 *   character:<id>                                                 one document per character of the registry (`characters` = its own fields)
 *   (`map` = the SCUTOONS world map's own settings – size, layers –, its places and routes are documents of their own:
 *   agents/MAP.md, agents/MAP-DOCUMENTS.md, WorldMapDocs; `locations:*` / `routes:*` are the old per-toon map, read-only)
 *
 *   item:<id>  recipe:<id>  album:<id>  product:<id>  edition:<id>
 *                                        a catalog entry as its own versioned document (agents/CATALOG-IN-CREATOR.md):
 *                                        drafts like content, publishing writes the catalog table (CatalogDocs); the
 *                                        toon is the entry's `toon` field, not a key segment (an entry may move)
 *
 * A chapter is ONE document: the scene documents (`scene:<toon>:<chapter>:<name>`, groups shared by a `ref`) were
 * inlined into their chapters and archived to entity_versions_legacy 2026-10-19 (migration 2026-10-19-inline-scenes).
 *
 * Every toon-related text of every locale lives in these documents (per-locale documents or `{ de, en }` fields); UI
 * chrome stays in the apps' message files.
 */
final class EntityKey
{
    /** kind → [has toon, has number, localized] */
    public const KINDS = [
        'toon' => [true, false, false],
        'toon-settings' => [true, false, true],
        'season' => [true, true, true],
        'chapter' => [true, true, true],
        'locations' => [true, false, true],
        'routes' => [true, false, true],
        'cast' => [true, false, false],
        'credits' => [true, false, false],
        'season-script' => [true, true, true],
        'chapter-script' => [true, true, true],
        // one segment: the subject (`character.catpope.human`), then the locale – parse()
        'kanon' => [false, false, true],
        'storyboard' => [true, true, false],
        'characters' => [false, false, false],
        'moods' => [false, false, false],
        'asset-requests' => [false, false, false],
        'people' => [false, false, false],
        'roles' => [false, false, false],
        'events' => [false, false, false],
        'map' => [false, false, false],
        'colorations' => [false, false, false],
        // one segment: the id (ID_KINDS)
        'place' => [false, false, false],
        'route' => [false, false, false],
        'region' => [false, false, false],
        'character' => [false, false, false],
    ];
    /**
     * Kinds addressed by an id of their own (`place:<id>`, `route:<id>`; platform-wide, language-neutral – agents/MAP-DOCUMENTS.md):
     * the id is a slug, stable forever once published (events reference it). Meant for the catalog's items too (`item:<id>`).
     */
    public const ID_KINDS = ['place', 'route', 'region', 'character'];
    public const ID_PATTERN = '#^[a-z0-9][a-z0-9_-]{0,63}$#';
    /**
     * Kinds that are working material of the creators, never content for readers: no library endpoint reads them (only
     * `/api/entities`, role creator) and the production seed (`import-content --sql`) leaves them out unless `--only`
     * names them (agents/DATA.md "Script"). The storyboards (#120) too: their working material, never a reader's.
     */
    public const CREATOR_ONLY_KINDS = ['storyboard', 'season-script', 'chapter-script', 'kanon'];
    /** a Kanon text's subject segment (no id holds a dot, so a key never reads two ways; @scutoons/content KANON_SUBJECT) */
    public const KANON_SUBJECT_PATTERN = '#^(?:toon\.[a-z0-9_-]{1,64}|place\.[a-z0-9][a-z0-9_-]{0,63}|character\.[a-z0-9][a-z0-9_-]{0,63}(?:\.[a-z0-9][a-z0-9-]{0,63}){0,2})$#';
    /** document kind → catalog kind (CatalogStore::KINDS) of the catalog documents `<kind>:<id>` */
    public const CATALOG_KINDS = ['item' => 'items', 'recipe' => 'recipes', 'album' => 'albums', 'product' => 'products', 'edition' => 'editions', 'home-scene' => 'home-scenes', 'airlift-carrier' => 'airlift-carriers',
        'instrument' => 'instruments', 'preset' => 'presets', 'rig' => 'rigs', 'module' => 'modules'];
    /**
     * a catalog document's id: the widest catalog id rule (CatalogValidator checks the kind's own rule on save) – dots
     * for presets (`cathedral_voice.bright`) and rigs (`rig:character.catpope`); one segment, so a key never reads two ways
     */
    public const CATALOG_ID_PATTERN = '#^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$#';
    /**
     * Localized kinds stored as one language-neutral document under the entity itself (no locale segment, agents/NEUTRAL.md,
     * NeutralDoc); their per-locale keys are the legacy form, still accepted until production is converted
     */
    public const NEUTRAL_DOC_KINDS = ['chapter', 'season'];
    public const TOON_PATTERN = '#^[a-z0-9_-]{1,64}$#';
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    /** season numbers start at 1; chapter numbers at 0 (chapter 0 = a season's prologue, e.g. a music video) */
    public const SEASON_NUMBER_PATTERN = '#^[1-9][0-9]{0,5}$#';
    public const CHAPTER_NUMBER_PATTERN = '#^(0|[1-9][0-9]{0,5})$#';

    public function __construct(
        public readonly string $kind,
        public readonly ?string $toon = null,
        public readonly ?int $number = null,
        public readonly ?string $locale = null,
        /** the id of an ID_KINDS key (`place:cathedral` → cathedral) or a catalog document's entry id (`item:camera` → camera) */
        public readonly ?string $id = null,
    ) {
    }

    public static function parse(mixed $key): self
    {
        $invalid = new ApiError(400, 'invalid_key', 'Unknown entity key.');
        if (!is_string($key) || strlen($key) > 191) throw $invalid;
        $parts = explode(':', $key);
        $kind = array_shift($parts);
        if (isset(self::CATALOG_KINDS[$kind])) {
            if (count($parts) !== 1 || !preg_match(self::CATALOG_ID_PATTERN, $parts[0])) throw $invalid;
            return new self($kind, id: $parts[0]);
        }
        $shape = self::KINDS[$kind] ?? null;
        if ($shape === null) throw $invalid;
        if (in_array($kind, self::ID_KINDS, true)) {
            if (count($parts) !== 1 || !preg_match(self::ID_PATTERN, $parts[0])) throw $invalid;
            return new self((string) $kind, id: $parts[0]);
        }
        if ($kind === 'kanon') {
            if (count($parts) !== 2 || !preg_match(self::KANON_SUBJECT_PATTERN, $parts[0]) || !preg_match(self::LOCALE_PATTERN, $parts[1])) throw $invalid;
            $toon = str_starts_with($parts[0], 'toon.') ? substr($parts[0], 5) : null;
            if ($toon === 'scutoons') throw $invalid;
            return new self('kanon', $toon, null, $parts[1], $parts[0]);
        }
        [$hasToon, $hasNumber, $localized] = $shape;
        $expected = (int) $hasToon + (int) $hasNumber + (int) $localized;
        if ($localized && count($parts) === $expected - 1 && in_array($kind, self::NEUTRAL_DOC_KINDS, true)) $localized = false; // the neutral document
        elseif (count($parts) !== $expected) throw $invalid;
        $toon = $hasToon ? array_shift($parts) : null;
        $number = $hasNumber ? array_shift($parts) : null;
        $locale = $localized ? array_shift($parts) : null;
        if ($toon !== null && (!preg_match(self::TOON_PATTERN, $toon) || $toon === 'scutoons')) throw $invalid;
        if ($number !== null && !preg_match(in_array($kind, ['season', 'season-script'], true) ? self::SEASON_NUMBER_PATTERN : self::CHAPTER_NUMBER_PATTERN, $number)) throw $invalid;
        if ($locale !== null && !preg_match(self::LOCALE_PATTERN, $locale)) throw $invalid;
        return new self((string) $kind, $toon, $number === null ? null : (int) $number, $locale);
    }

    public static function isValid(mixed $key): bool
    {
        try {
            self::parse($key);
            return true;
        } catch (ApiError) {
            return false;
        }
    }

    public function __toString(): string
    {
        if ($this->kind === 'kanon') return $this->entity() . ":$this->locale";
        return implode(':', array_filter([$this->kind, $this->toon, $this->number === null ? null : (string) $this->number, $this->locale, $this->id],
            fn($p) => $p !== null));
    }

    /** A catalog entry's document (`item:<id>` …) */
    public function isCatalog(): bool
    {
        return isset(self::CATALOG_KINDS[$this->kind]);
    }

    /** The catalog kind of a catalog document (`items` …) */
    public function catalogKind(): ?string
    {
        return self::CATALOG_KINDS[$this->kind] ?? null;
    }

    /** A language-neutral document: a kind without locales, or the neutral document of a chapter / season */
    public function isNeutral(): bool
    {
        return $this->locale === null;
    }

    /** The neutral document of a chapter / season (not a legacy per-locale one) */
    public function isNeutralDoc(): bool
    {
        return $this->locale === null && in_array($this->kind, self::NEUTRAL_DOC_KINDS, true);
    }

    /** The same entity in another locale (localized kinds) */
    public function withLocale(string $locale): self
    {
        return new self($this->kind, $this->toon, $this->number, $locale, $this->id);
    }

    /** Key without the locale (the entity across its languages), e.g. `chapter:catpope:2` */
    public function entity(): string
    {
        if ($this->kind === 'kanon') return "kanon:$this->id";
        return implode(':', array_filter([$this->kind, $this->toon, $this->number === null ? null : (string) $this->number, $this->id], fn($p) => $p !== null));
    }
}
