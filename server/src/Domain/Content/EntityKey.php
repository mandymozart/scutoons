<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;

/**
 * The key of a stored entity (agents/DATA.md §2 "Entity keys"): `<kind>[:<toon>][:<number>][:<scene>][:<locale>]`.
 *
 *   toon:<toon>                          the toon's index entry (list entry, status, languages, names, the toon settings – texts per language)
 *   toon-settings:<toon>:<l>             legacy: the toon's per-language settings (title, description, release, status) – since agents/NEUTRAL.md step 4 they
 *                                        are locale maps in toon:<toon> (with the language-neutral ones: map, tile …); read while the entry has no texts
 *   season:<toon>:<n>                    chapter:<toon>:<n>            scene:<toon>:<chapter>:<name>
 *                                        one language-neutral document each, texts as locale maps (NeutralDoc, agents/NEUTRAL.md)
 *   season:<toon>:<n>:<l>  chapter:<toon>:<n>:<l>  scene:<toon>:<chapter>:<name>:<l>
 *                                        the legacy per-locale documents: read while the entity has no neutral document
 *   locations:<toon>:<l>  routes:<toon>:<l>                            the map
 *   cast:<toon>  credits:<toon>                                        language-neutral, per toon
 *   script:<toon>:<l>                    the toon's screenplay per locale (creator material, CREATOR_ONLY_KINDS)
 *   characters  moods  asset-requests  people  roles  events  map      platform-wide, language-neutral
 *   place:<id>  route:<id>                                         one document per place / route of the world map (ID_KINDS)
 *   (`map` = the SCUTOONS world map's own settings – size, layers –, its places and routes are documents of their own:
 *   agents/MAP.md, agents/MAP-DOCUMENTS.md, WorldMapDocs; `locations:*` / `routes:*` are the old per-toon map, read-only)
 *
 * Every toon-related text of every locale lives in these documents (per-locale documents or `{ de, en }` fields); UI
 * chrome stays in the apps' message files; the catalog (items, editions, products) follows later (DATA.md "B2").
 */
final class EntityKey
{
    /** kind → [has toon, has number, has scene, localized] */
    public const KINDS = [
        'toon' => [true, false, false, false],
        'toon-settings' => [true, false, false, true],
        'season' => [true, true, false, true],
        'chapter' => [true, true, false, true],
        'scene' => [true, true, true, true],
        'locations' => [true, false, false, true],
        'routes' => [true, false, false, true],
        'cast' => [true, false, false, false],
        'credits' => [true, false, false, false],
        'script' => [true, false, false, true],
        'characters' => [false, false, false, false],
        'moods' => [false, false, false, false],
        'asset-requests' => [false, false, false, false],
        'people' => [false, false, false, false],
        'roles' => [false, false, false, false],
        'events' => [false, false, false, false],
        'map' => [false, false, false, false],
        // one segment: the id (ID_KINDS)
        'place' => [false, false, false, false],
        'route' => [false, false, false, false],
    ];
    /**
     * Kinds addressed by an id of their own (`place:<id>`, `route:<id>`; platform-wide, language-neutral – agents/MAP-DOCUMENTS.md):
     * the id is a slug, stable forever once published (events reference it). Meant for the catalog's items too (`item:<id>`).
     */
    public const ID_KINDS = ['place', 'route'];
    public const ID_PATTERN = '#^[a-z0-9][a-z0-9_-]{0,63}$#';
    /**
     * Kinds that are working material of the creators, never content for readers: no library endpoint reads them (only
     * `/api/entities`, role creator) and the production seed (`import-content --sql`) leaves them out unless `--only`
     * names them (agents/DATA.md "Script").
     */
    public const CREATOR_ONLY_KINDS = ['script'];
    /**
     * Localized kinds stored as one language-neutral document under the entity itself (no locale segment, agents/NEUTRAL.md,
     * NeutralDoc); their per-locale keys are the legacy form, still accepted until production is converted
     */
    public const NEUTRAL_DOC_KINDS = ['chapter', 'scene', 'season'];
    public const TOON_PATTERN = '#^[a-z0-9_-]{1,64}$#';
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    public const SCENE_PATTERN = '#^[a-z0-9_-]{1,64}$#';
    /** season numbers start at 1; chapter numbers at 0 (chapter 0 = a season's prologue, e.g. a music video) */
    public const SEASON_NUMBER_PATTERN = '#^[1-9][0-9]{0,5}$#';
    public const CHAPTER_NUMBER_PATTERN = '#^(0|[1-9][0-9]{0,5})$#';

    public function __construct(
        public readonly string $kind,
        public readonly ?string $toon = null,
        public readonly ?int $number = null,
        public readonly ?string $scene = null,
        public readonly ?string $locale = null,
        public readonly ?string $id = null,
    ) {
    }

    public static function parse(mixed $key): self
    {
        $invalid = new ApiError(400, 'invalid_key', 'Unknown entity key.');
        if (!is_string($key) || strlen($key) > 191) throw $invalid;
        $parts = explode(':', $key);
        $kind = array_shift($parts);
        $shape = self::KINDS[$kind] ?? null;
        if ($shape === null) throw $invalid;
        if (in_array($kind, self::ID_KINDS, true)) {
            if (count($parts) !== 1 || !preg_match(self::ID_PATTERN, $parts[0])) throw $invalid;
            return new self((string) $kind, id: $parts[0]);
        }
        [$hasToon, $hasNumber, $hasScene, $localized] = $shape;
        $expected = (int) $hasToon + (int) $hasNumber + (int) $hasScene + (int) $localized;
        if ($localized && count($parts) === $expected - 1 && in_array($kind, self::NEUTRAL_DOC_KINDS, true)) $localized = false; // the neutral document
        elseif (count($parts) !== $expected) throw $invalid;
        $toon = $hasToon ? array_shift($parts) : null;
        $number = $hasNumber ? array_shift($parts) : null;
        $scene = $hasScene ? array_shift($parts) : null;
        $locale = $localized ? array_shift($parts) : null;
        if ($toon !== null && (!preg_match(self::TOON_PATTERN, $toon) || $toon === 'scutoons')) throw $invalid;
        if ($number !== null && !preg_match($kind === 'season' ? self::SEASON_NUMBER_PATTERN : self::CHAPTER_NUMBER_PATTERN, $number)) throw $invalid;
        if ($scene !== null && !preg_match(self::SCENE_PATTERN, $scene)) throw $invalid;
        if ($locale !== null && !preg_match(self::LOCALE_PATTERN, $locale)) throw $invalid;
        return new self((string) $kind, $toon, $number === null ? null : (int) $number, $scene, $locale);
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
        return implode(':', array_filter([$this->kind, $this->toon, $this->number === null ? null : (string) $this->number, $this->scene, $this->locale, $this->id],
            fn($p) => $p !== null));
    }

    /** A language-neutral document: a kind without locales, or the neutral document of a chapter / scene / season */
    public function isNeutral(): bool
    {
        return $this->locale === null;
    }

    /** The neutral document of a chapter / scene / season (not a legacy per-locale one) */
    public function isNeutralDoc(): bool
    {
        return $this->locale === null && in_array($this->kind, self::NEUTRAL_DOC_KINDS, true);
    }

    /** The same entity in another locale (localized kinds) */
    public function withLocale(string $locale): self
    {
        return new self($this->kind, $this->toon, $this->number, $this->scene, $locale, $this->id);
    }

    /** Key without the locale (the entity across its languages), e.g. `chapter:catpope:2` */
    public function entity(): string
    {
        return implode(':', array_filter([$this->kind, $this->toon, $this->number === null ? null : (string) $this->number, $this->scene, $this->id], fn($p) => $p !== null));
    }

    /** A chapter's scene reference (`ref.src` of a group: `chapter<N>/<name>.json`, the content format) → its entity */
    public static function sceneOf(string $toon, string $src): ?string
    {
        return preg_match('#^chapter(0|[1-9][0-9]{0,5})/([a-z0-9_-]{1,64})\.json$#', $src, $m) === 1 ? "scene:$toon:$m[1]:$m[2]" : null;
    }
}
