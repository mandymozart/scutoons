<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The reader's entity service (agents/DATA.md §3, docs/API.md "Library"): toons, seasons, chapters with their scenes
 * inlined, cast, credits and events, composed from the content store's documents (`entity_versions`, keyed by entity
 * key – the configuration format of @scutoons/content stays the format of every document).
 *
 * - Locale: the asked one, else the toon's source language, else another language of the toon (per document).
 * - Visibility (the viewer's rules, now on the server): toon index entry + toon settings must be public, a chapter's
 *   status is the season's `sequence[].status`; `coming-soon` → 403 coming_soon, `draft` / not listed → 403 not_public.
 *   CONTENT_SHOW_DRAFTS makes `draft` readable (local development). Draft mode (creators) reads the newest versions and
 *   skips every gate.
 * - Cache: answers are cached under endpoint + snapshot fingerprint (all version ids involved), so a publish
 *   changes the key – no invalidation. Drafts are never cached.
 */
final class Library
{
    /** Not a toon id (the old pseudo-toon of platform-wide documents) */
    public const PLATFORM = 'scutoons';
    public const PUBLISHED = 'published';
    public const DRAFT = 'draft';
    public const COMING_SOON = 'coming-soon';
    public const DEFAULT_SOURCE = 'de';
    /** Toons for development and the creator only (the sandbox): never in the public toons list, never in the production seed */
    public const DEV_ONLY_TOONS = ['sandbox'];
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    /**
     * Toon settings that are the same in every language (owner 2026-10-01): stored once in the toon's index entry
     * `toon:<toon>`, merged into the bundle's per-language `toon` settings; @scutoons/content TOON_NEUTRAL_FIELDS.
     * `cover` was one 2026-10-01 – 2026-10-02; covers belong to the season now (`season.cover`, owner: a toon is shown with
     * its avatar). An older per-language toon `cover` still passes through (the reader's fallback).
     */
    public const TOON_NEUTRAL_FIELDS = ['authors', 'chapters', 'thumbnail', 'map', 'announce', 'tile'];
    /** Bump when the shape of an answer changes: old cache entries are never read again */
    private const FORMAT = 4;
    private const CACHE_TTL = 604800;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;
    /** @scutoons/content LEGACY_CHARACTER_IDS: old character ids in scene refs → the registry character */
    private const LEGACY_CHARACTER_IDS = ['plf' => 'catpope', 'cat' => 'catpope', 'mountain-tiger' => 'catpope'];

    public function __construct(
        private readonly Database $db,
        private readonly Cache $cache,
        private readonly Settings $settings,
    ) {
    }

    public function showDrafts(): bool
    {
        return $this->settings->bool('CONTENT_SHOW_DRAFTS');
    }

    // ── Endpoints ─────────────────────────────────────────────────────────────────────────────────────────

    /**
     * GET /library/toons → ToonIndex[]: published entries + announced ones (`announce: true` in the index entry, marked
     * `announced: true` – their pages stay 404, so no season or chapter of them is exposed); drafts too with
     * CONTENT_SHOW_DRAFTS; all in draft mode. The dev-only toons (sandbox) are never listed publicly. Every entry carries
     * its landing `tile` when there is one; the other language-neutral settings (map …) are left out of the list.
     * Entries stored before the neutral settings moved to the index read `announce` / `tile` from the toon settings.
     */
    public function toons(bool $draft): Answer
    {
        $snapshot = Snapshot::toonIndex($this->db, $draft);
        return $this->answer('toons', $snapshot, function () use ($snapshot, $draft): array {
            $list = [];
            foreach ($snapshot->ordered() as $key) {
                if (!str_starts_with($key, 'toon:')) continue;
                $entry = $snapshot->doc($key);
                if (!$entry instanceof stdClass) continue;
                $toon = substr($key, strlen('toon:'));
                $settings = $this->listSettings($snapshot, $toon, $entry);
                $announce = property_exists($entry, 'announce') ? $entry->announce : ($settings->announce ?? null);
                $tile = property_exists($entry, 'tile') ? $entry->tile : ($settings->tile ?? null);
                $status = $entry->status ?? null;
                $announced = $status !== self::PUBLISHED && $announce === true;
                $public = $status === self::PUBLISHED || $announced;
                $visible = $draft || ($status === self::DRAFT && $this->showDrafts())
                    || ($public && !in_array($toon, self::DEV_ONLY_TOONS, true));
                if (!$visible) continue;
                $entry = self::indexEntry($entry);
                if ($announced) $entry->announced = true;
                if ($tile instanceof stdClass) $entry->tile = $tile;
                $list[] = $entry;
            }
            return [$list, null];
        });
    }

    /** Older data: the toon settings the list reads `announce` and `tile` from when the index entry lacks them (source language's, else any) */
    private function listSettings(Snapshot $snapshot, string $toon, stdClass $entry): ?stdClass
    {
        [$source] = self::languages($entry, []);
        $keys = $snapshot->keys("toon-settings:$toon:");
        $preferred = "toon-settings:$toon:$source";
        if (in_array($preferred, $keys, true)) array_unshift($keys, $preferred);
        foreach ($keys as $key) {
            $settings = $snapshot->doc($key);
            if ($settings instanceof stdClass) return $settings;
        }
        return null;
    }

    /** The index entry without the language-neutral toon settings it stores (the shape of `ToonIndex` in the answers) */
    private static function indexEntry(stdClass $entry): stdClass
    {
        $entry = clone $entry;
        foreach (self::TOON_NEUTRAL_FIELDS as $field) unset($entry->$field);
        return $entry;
    }

    /** The reader's `Toon`: the per-language settings + the language-neutral ones of the index entry (the entry wins) */
    private static function withNeutralSettings(stdClass $settings, ?stdClass $entry): stdClass
    {
        $settings = clone $settings;
        if ($entry === null) return $settings;
        foreach (self::TOON_NEUTRAL_FIELDS as $field) {
            if (property_exists($entry, $field)) $settings->$field = $entry->$field;
        }
        return $settings;
    }

    /**
     * GET /library/toons/{toon}?locale= → { entry, toon, seasons: SeasonOverview[], locations, routes }; `toon` = the
     * locale's settings with the language-neutral ones of the index entry merged in, `entry` = the index entry without them
     */
    public function toon(string $toon, ?string $locale, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        $locale ??= $ctx->source;
        return $this->answer("toon:$toon:$locale", $ctx->snapshot, function () use ($ctx, $locale): array {
            [$served, $toonDoc] = $this->visibleToon($ctx, $locale);
            $toon = $ctx->toon;
            $seasons = [];
            foreach ($this->seasonNumbers($ctx) as $number) {
                $overview = $this->seasonOverview($ctx, $number, $locale);
                if ($overview !== null) $seasons[] = $overview;
            }
            // the map: computed from the world map (entity `map`) in the old per-toon shape for older viewer builds; the
            // old `locations:<toon>:<l>` / `routes:<toon>:<l>` documents only while no world map is stored
            $world = $ctx->snapshot->doc('map');
            if ($world instanceof stdClass) {
                [$locations, $routes] = self::legacyMapLists($world, $toon, $locale);
            } else {
                $locations = $ctx->pick("locations:$toon", $locale)[1] ?? [];
                $routes = $ctx->pick("routes:$toon", $locale)[1] ?? [];
            }
            return [[
                'entry' => $ctx->entry !== null ? self::indexEntry($ctx->entry) : null,
                'toon' => self::withLegacyMap(self::withNeutralSettings($toonDoc, $ctx->entry), $world instanceof stdClass ? $world : null, $toon),
                'seasons' => $seasons,
                'locations' => $locations,
                'routes' => $routes,
            ], $served];
        }, $ctx);
    }

    /** GET /library/toons/{toon}/seasons/{n}?locale= → SeasonOverview */
    public function season(string $toon, int $number, ?string $locale, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        $locale ??= $ctx->source;
        return $this->answer("season:$toon:$number:$locale", $ctx->snapshot, function () use ($ctx, $number, $locale): array {
            $this->visibleToon($ctx, $locale);
            $overview = $this->seasonOverview($ctx, $number, $locale)
                ?? throw new ApiError(404, 'not_found', "Toon $ctx->toon has no season $number.");
            return [$overview, $ctx->servedLocale];
        }, $ctx);
    }

    /** GET /library/toons/{toon}/chapters/{n}?locale= → Chapter with every `ref` group replaced by its scene file */
    public function chapter(string $toon, int $number, ?string $locale, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        $locale ??= $ctx->source;
        return $this->answer("chapter:$toon:$number:$locale", $ctx->snapshot, function () use ($ctx, $number, $locale): array {
            $this->visibleToon($ctx, $locale);
            if (!$ctx->draft) {
                $status = $this->chapterStatus($ctx, $number, $locale);
                if ($status === self::COMING_SOON) {
                    $picked = $ctx->pick("chapter:$ctx->toon:$number", $locale);
                    throw new ApiError(403, 'coming_soon', "Chapter $number is coming soon.",
                        ['chapter' => self::summary($picked[1] ?? null, 'chapter', $number, $status)]);
                }
                if ($status === null ? !$ctx->includeDrafts : !$ctx->isPublic($status)) {
                    throw new ApiError(403, 'not_public', "Chapter $number is not published.");
                }
            }
            $ctx->fallback = false; // the answer's fallback flag = the chapter + its scenes
            $picked = $ctx->pick("chapter:$ctx->toon:$number", $locale)
                ?? throw new ApiError(404, 'not_found', "Chapter $number of $ctx->toon not found.");
            [$served, $chapter] = $picked;
            if (!$chapter instanceof stdClass) throw new ApiError(404, 'not_found', "Chapter $number of $ctx->toon not found.");
            return [$this->withScenes($ctx, $chapter, $served), $served];
        }, $ctx);
    }

    /**
     * GET /library/toons/{toon}/cast[?chapter=N] → { registry, cast }: the character registry reduced to the toon's cast
     * (or to the characters the chapter's events reference) + the toon's cast; null where one is missing.
     * Language-neutral: the client resolves names/images per locale with resolveCast().
     */
    public function cast(string $toon, ?int $chapter, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        return $this->answer("cast:$toon:" . ($chapter ?? '*'), $ctx->snapshot, function () use ($ctx, $chapter): array {
            $this->visibleToon($ctx, $ctx->source);
            $registry = $ctx->snapshot->doc('characters');
            $cast = self::withStar($ctx->snapshot->doc("cast:$ctx->toon"), self::star($registry, $ctx->toon));
            if ($chapter !== null) {
                $doc = $ctx->pick("chapter:$ctx->toon:$chapter", $ctx->source);
                if ($doc === null || !$doc[1] instanceof stdClass) throw new ApiError(404, 'not_found', "Chapter $chapter of $ctx->toon not found.");
                $ids = self::referencedCharacters($this->withScenes($ctx, $doc[1], $doc[0]));
                // the chapter's `featuring` (reader: avatars at the chapter start)
                foreach (is_array($doc[1]->featuring ?? null) ? $doc[1]->featuring : [] as $id) {
                    if (is_string($id)) $ids[$id] = true;
                }
            } else {
                $ids = [];
                foreach ((array) ($cast->seasons ?? []) as $entries) {
                    foreach (is_array($entries) ? $entries : [] as $entry) {
                        if (isset($entry->character) && is_string($entry->character)) $ids[$entry->character] = true;
                    }
                }
            }
            if ($registry instanceof stdClass && is_array($registry->characters ?? null)) {
                $registry = clone $registry;
                $registry->characters = array_values(array_filter($registry->characters,
                    fn($c) => is_object($c) && isset($c->id) && isset($ids[(string) $c->id])));
            } else {
                $registry = null;
            }
            $ctx->fallback = false;
            return [['registry' => $registry, 'cast' => $cast instanceof stdClass ? $cast : null], null];
        }, $ctx);
    }

    /** GET /library/toons/{toon}/credits → { credits, people (referenced only), roles }; 404 without credits */
    public function credits(string $toon, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        return $this->answer("credits:$toon", $ctx->snapshot, function () use ($ctx): array {
            $this->visibleToon($ctx, $ctx->source);
            $credits = $ctx->snapshot->doc("credits:$ctx->toon");
            if (!$credits instanceof stdClass) throw new ApiError(404, 'not_found', "Toon $ctx->toon has no credits.");
            $people = $ctx->snapshot->doc('people');
            if ($people instanceof stdClass && is_array($people->people ?? null)) {
                $people = clone $people;
                $people->people = self::referencedPeople($credits, $people->people);
            } else {
                $people = null;
            }
            $roles = $ctx->snapshot->doc('roles');
            $ctx->fallback = false;
            return [['credits' => $credits, 'people' => $people, 'roles' => $roles instanceof stdClass ? $roles : null], null];
        }, $ctx);
    }

    /** GET /library/characters → the whole character registry (e.g. every A-pose body for the reader's avatar picker) */
    public function characters(bool $draft): Answer
    {
        $snapshot = Snapshot::platform($this->db, $draft);
        return $this->answer('characters', $snapshot, function () use ($snapshot): array {
            $registry = $snapshot->doc('characters') ?? throw new ApiError(404, 'not_found', 'No characters stored.');
            return [$registry, null];
        });
    }

    /** GET /events → the stored events document (entity `events`) */
    public function events(bool $draft): Answer
    {
        $snapshot = Snapshot::platform($this->db, $draft);
        return $this->answer('events', $snapshot, function () use ($snapshot): array {
            $events = $snapshot->doc('events')
                ?? throw new ApiError(404, 'not_found', 'No events stored.');
            return [$events, null];
        });
    }

    /**
     * GET /library/map?locale=&toon= → the SCUTOONS world map for this reader (agents/MAP.md §2.3): { width, height,
     * layers, locations, routes[, view] } with every name resolved to the locale (else the source language `de`, else any;
     * X-Content-Fallback when one was missing). Only the places / routes of toons the reader may see: published, not dev
     * only (the sandbox) – drafts with CONTENT_SHOW_DRAFTS, everything in draft mode; announced toons' places stay out
     * until the toon is published. Hidden places are sent (the unlock state is the reader's, client-side). `toon` adds
     * `view` = where the map opens for that toon (its start view in `toon:<toon>`, else the centre of its places; 404 when
     * the toon is not visible) and `view.color` = the toon's colour for the sea (tile colour, else its star's colour).
     */
    public function map(?string $locale, ?string $toon, bool $draft): Answer
    {
        if ($toon !== null && (!preg_match(EntityKey::TOON_PATTERN, $toon) || $toon === self::PLATFORM)) {
            throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        }
        $locale ??= self::DEFAULT_SOURCE;
        $snapshot = Snapshot::kinds($this->db, ['map', 'toon', 'characters'], $draft);
        $ctx = new Context($snapshot, self::PLATFORM, null, self::DEFAULT_SOURCE, [self::DEFAULT_SOURCE], $draft, $draft || $this->showDrafts());
        return $this->answer("map:$locale:" . ($toon ?? '*'), $snapshot, function () use ($snapshot, $ctx, $locale, $toon, $draft): array {
            $world = $snapshot->doc('map');
            if (!$world instanceof stdClass) throw new ApiError(404, 'not_found', 'No world map stored.');
            $visible = [];
            $isVisible = function (mixed $id) use (&$visible, $snapshot, $draft): bool {
                if (!is_string($id)) return false;
                if (!array_key_exists($id, $visible)) {
                    $entry = $snapshot->doc("toon:$id");
                    $status = $entry instanceof stdClass ? ($entry->status ?? null) : null;
                    $visible[$id] = $entry instanceof stdClass && ($draft
                        || ($status === self::DRAFT && $this->showDrafts())
                        || ($status === self::PUBLISHED && !in_array($id, self::DEV_ONLY_TOONS, true)));
                }
                return $visible[$id];
            };
            $anyVisible = fn(mixed $toons) => is_array($toons) && array_filter($toons, $isVisible) !== [];
            $name = function (mixed $text) use ($locale, $ctx): string {
                [$value, $fallback] = self::mapName($text, $locale);
                if ($fallback) $ctx->fallback = true;
                return $value;
            };
            $places = [];
            foreach (is_array($world->locations ?? null) ? $world->locations : [] as $place) {
                if (!$place instanceof stdClass || !$anyVisible($place->toons ?? null)) continue;
                $place = clone $place;
                $place->name = $name($place->name ?? null);
                $places[] = $place;
            }
            $routes = [];
            foreach (is_array($world->routes ?? null) ? $world->routes : [] as $route) {
                if (!$route instanceof stdClass || !$anyVisible($route->toons ?? null)) continue;
                $route = clone $route;
                $route->name = $name($route->name ?? null);
                $route->waypoints = array_map(function (mixed $w) use ($name) {
                    if (!$w instanceof stdClass || !property_exists($w, 'name')) return $w;
                    $w = clone $w;
                    $w->name = $name($w->name);
                    return $w;
                }, is_array($route->waypoints ?? null) ? $route->waypoints : []);
                $routes[] = $route;
            }
            $answer = [
                'width' => $world->width ?? 0,
                'height' => $world->height ?? 0,
                'layers' => is_array($world->layers ?? null) ? $world->layers : [],
                'locations' => $places,
                'routes' => $routes,
            ];
            if ($toon !== null) {
                if (!$isVisible($toon)) throw new ApiError(404, 'not_found', "Unknown toon $toon.");
                $entry = $snapshot->doc("toon:$toon");
                $view = $entry instanceof stdClass && ($entry->map ?? null) instanceof stdClass ? $entry->map : new stdClass();
                $center = $view->center ?? null;
                $answer['view'] = [
                    'center' => $center instanceof stdClass && is_numeric($center->x ?? null) && is_numeric($center->y ?? null)
                        ? ['x' => $center->x, 'y' => $center->y]
                        : self::centerOfPlaces($world, $toon),
                    'zoom' => is_numeric($view->zoom ?? null) && $view->zoom > 0 ? $view->zoom : 1,
                ];
                // the sea is the toon's colour (owner 2026-10-01): its landing tile colour, else its star's registry colour
                // (@scutoons/content toonTileColor); none → the reader's default
                $tile = $entry instanceof stdClass && ($entry->tile ?? null) instanceof stdClass ? $entry->tile : null;
                $color = $tile !== null && is_string($tile->color ?? null) && $tile->color !== '' ? $tile->color : null;
                if ($color === null) {
                    $registry = $snapshot->doc('characters');
                    foreach ($registry instanceof stdClass && is_array($registry->characters ?? null) ? $registry->characters : [] as $c) {
                        if ($c instanceof stdClass && ($c->own_toon ?? null) === $toon && is_string($c->color ?? null) && $c->color !== '') {
                            $color = $c->color;
                            break;
                        }
                    }
                }
                if ($color !== null) $answer['view']['color'] = $color;
            }
            return [$answer, $locale];
        }, $ctx);
    }

    /**
     * A `{ de, en }` name in a locale: the locale, else the source language, else any non-empty one
     * @return array{0: string, 1: bool} [name, whether it fell back]
     */
    private static function mapName(mixed $text, string $locale): array
    {
        if (is_string($text)) return [$text, false];
        if (!$text instanceof stdClass) return ['', false];
        foreach ([$locale, self::DEFAULT_SOURCE] as $i => $l) {
            $value = $text->{$l} ?? null;
            if (is_string($value) && trim($value) !== '') return [$value, $i > 0];
        }
        foreach (get_object_vars($text) as $value) {
            if (is_string($value) && trim($value) !== '') return [$value, true];
        }
        return ['', false];
    }

    /** @return array{x: int|float, y: int|float} the centre of the bounding box of a toon's places (else the world's centre) */
    private static function centerOfPlaces(stdClass $world, string $toon): array
    {
        $xs = $ys = [];
        foreach (is_array($world->locations ?? null) ? $world->locations : [] as $place) {
            if (!$place instanceof stdClass || !in_array($toon, is_array($place->toons ?? null) ? $place->toons : [], true)) continue;
            if (is_numeric($place->x ?? null) && is_numeric($place->y ?? null)) {
                $xs[] = $place->x;
                $ys[] = $place->y;
            }
        }
        if ($xs === [] || $ys === []) return ['x' => ($world->width ?? 0) / 2, 'y' => ($world->height ?? 0) / 2];
        return ['x' => (min($xs) + max($xs)) / 2, 'y' => (min($ys) + max($ys)) / 2];
    }

    /**
     * The toon's places and routes in the shape of the old `locations:<toon>:<l>` / `routes:<toon>:<l>` documents, for
     * older viewer builds (one release, then dropped; @scutoons/content `legacyToonMap`): every place / route whose `toons`
     * has the toon, hidden ones too (old viewers have no unlock state and always showed every place); a route lists its
     * place waypoints (free points did not exist), unknown ids included, as before.
     * @return array{0: list<array<string, mixed>>, 1: list<array<string, mixed>>}
     */
    private static function legacyMapLists(stdClass $world, string $toon, string $locale): array
    {
        $has = fn(mixed $item) => $item instanceof stdClass && in_array($toon, is_array($item->toons ?? null) ? $item->toons : [], true);
        $locations = [];
        foreach (is_array($world->locations ?? null) ? $world->locations : [] as $place) {
            if (!$has($place)) continue;
            $chapter = $place->chapter ?? null;
            $location = [
                'id' => $place->id ?? '',
                'label' => self::mapName($place->name ?? null, $locale)[0],
                'toonId' => $chapter instanceof stdClass && is_string($chapter->toon ?? null) ? $chapter->toon : $toon,
                'type' => 'chapter',
            ];
            if ($chapter instanceof stdClass && isset($chapter->number)) $location['chapterNumber'] = (string) $chapter->number;
            $location['image'] = ($place->image ?? null) instanceof stdClass && is_string($place->image->src ?? null) ? $place->image->src : '';
            $location['coordinates'] = ['x' => $place->x ?? 0, 'y' => $place->y ?? 0];
            $locations[] = $location;
        }
        $routes = [];
        foreach (is_array($world->routes ?? null) ? $world->routes : [] as $route) {
            if (!$has($route)) continue;
            $ids = [];
            foreach (is_array($route->waypoints ?? null) ? $route->waypoints : [] as $w) {
                if ($w instanceof stdClass && is_string($w->location ?? null)) $ids[] = $w->location;
            }
            $routes[] = ['id' => $route->id ?? '', 'title' => self::mapName($route->name ?? null, $locale)[0], 'locations' => $ids];
        }
        return [$locations, $routes];
    }

    /**
     * The old `toon.map` ({ layers: { terrain }, width, height, center }) for older viewer builds: the world's size and
     * terrain (path relative to the toon's folder, as before) + the toon's start view; only when the toon has one (the
     * entry's `map`). An entry stored before the world map (with `layers`) is passed through unchanged.
     */
    private static function withLegacyMap(stdClass $toonDoc, ?stdClass $world, string $toon): stdClass
    {
        $view = $toonDoc->map ?? null;
        if ($world === null || !$view instanceof stdClass || property_exists($view, 'layers')) return $toonDoc;
        $terrain = '';
        foreach (is_array($world->layers ?? null) ? $world->layers : [] as $layer) {
            if ($layer instanceof stdClass && is_string($layer->image ?? null) && ($terrain === '' || ($layer->id ?? null) === 'terrain')) {
                $terrain = $layer->image;
                if (($layer->id ?? null) === 'terrain') break;
            }
        }
        if ($terrain !== '') $terrain = str_starts_with($terrain, "$toon/") ? substr($terrain, strlen($toon) + 1) : "../$terrain";
        $center = $view->center ?? null;
        $toonDoc = clone $toonDoc;
        $toonDoc->map = [
            'layers' => ['terrain' => $terrain],
            'width' => $world->width ?? 0,
            'height' => $world->height ?? 0,
            'center' => $center instanceof stdClass && isset($center->x, $center->y) ? ['x' => $center->x, 'y' => $center->y] : self::centerOfPlaces($world, $toon),
        ];
        return $toonDoc;
    }

    // ── Composition ───────────────────────────────────────────────────────────────────────────────────────

    private function context(string $toon, bool $draft): Context
    {
        if (!preg_match(EntityKey::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        if ($toon === self::PLATFORM) throw new ApiError(404, 'not_found', "Unknown toon $toon.");
        $snapshot = Snapshot::toon($this->db, $toon, $draft);
        $entry = $snapshot->doc("toon:$toon");
        $entry = $entry instanceof stdClass ? $entry : null;
        $stored = [];
        foreach ($snapshot->keys("toon-settings:$toon:") as $key) $stored[] = substr($key, strlen("toon-settings:$toon:"));
        [$source, $languages] = self::languages($entry, $stored);
        return new Context($snapshot, $toon, $entry, $source, $languages, $draft, $draft || $this->showDrafts());
    }

    /**
     * toonLanguages() of @scutoons/content: `languages` → `i18n` keys → the locales the store has; source first
     * @param list<string> $stored
     * @return array{0: string, 1: list<string>}
     */
    private static function languages(?stdClass $entry, array $stored): array
    {
        $valid = fn(mixed $l) => is_string($l) && preg_match(self::LOCALE_PATTERN, $l) === 1;
        $entry ??= new stdClass();
        $listed = array_values(array_filter(is_array($entry->languages ?? null) ? $entry->languages : [], $valid));
        if ($listed === []) $listed = array_values(array_filter(array_keys((array) ($entry->i18n ?? [])), $valid));
        if ($listed === []) {
            sort($stored);
            $listed = in_array(self::DEFAULT_SOURCE, $stored, true) ? [self::DEFAULT_SOURCE, ...$stored] : $stored;
        }
        $source = $valid($entry->sourceLanguage ?? null) ? (string) $entry->sourceLanguage : ($listed[0] ?? self::DEFAULT_SOURCE);
        /** @var list<string> $ordered */
        $ordered = array_values(array_unique([$source, ...$listed]));
        return [$source, $ordered];
    }

    /**
     * The toon's settings when the toon is visible (index entry + settings public; production needs the index
     * entry), else 404 – like the viewer's toon layout
     * @return array{0: string, 1: stdClass}
     */
    private function visibleToon(Context $ctx, string $locale): array
    {
        $notFound = new ApiError(404, 'not_found', "Unknown toon $ctx->toon.");
        if (!$ctx->draft) {
            if ($ctx->entry === null ? !$ctx->includeDrafts : !$ctx->isPublic($ctx->entry->status ?? null)) throw $notFound;
        }
        $picked = $ctx->pick("toon-settings:$ctx->toon", $locale);
        if ($picked === null || !$picked[1] instanceof stdClass) throw $notFound;
        if (!$ctx->draft && !$ctx->isPublic($picked[1]->status ?? null)) throw $notFound;
        $ctx->servedLocale = $picked[0];
        return [$picked[0], $picked[1]];
    }

    /** @return list<int> season numbers the toon has in any locale */
    private function seasonNumbers(Context $ctx): array
    {
        $numbers = [];
        foreach ($ctx->snapshot->keys("season:$ctx->toon:") as $key) $numbers[(int) explode(':', $key)[2]] = true;
        $numbers = array_keys($numbers);
        sort($numbers);
        return $numbers;
    }

    /** SeasonOverview { number, season, chapters: ChapterSummary[] } – the toon layout's chapter list, or null */
    private function seasonOverview(Context $ctx, int $number, string $locale): ?stdClass
    {
        $picked = $ctx->pick("season:$ctx->toon:$number", $locale);
        if ($picked === null || !$picked[1] instanceof stdClass) return null;
        [$served, $season] = $picked;
        $chapters = [];
        foreach (is_array($season->sequence ?? null) ? $season->sequence : [] as $item) {
            $itemNumber = is_object($item) ? ($item->number ?? null) : null;
            if (!is_int($itemNumber) && !is_string($itemNumber)) continue;
            $status = $item->status ?? null;
            if (!$ctx->draft && !$ctx->isListed($status)) continue;
            $type = is_string($item->type ?? null) ? $item->type : 'chapter';
            $entity = "$type:$ctx->toon:$itemNumber";
            $chapter = $ctx->pick($entity, $served);
            if ($chapter === null) {
                if ($status !== self::COMING_SOON) $ctx->missing[] = $entity;
                $chapters[] = self::summary(null, $type, $itemNumber, $status);
                continue;
            }
            $chapters[] = self::summary($chapter[1], $type, $itemNumber, $status);
        }
        $overview = new stdClass();
        $overview->number = $number;
        $overview->season = $season;
        $overview->chapters = $chapters;
        return $overview;
    }

    /**
     * ChapterSummary: the chapter file without `groups`, `audio` and its deprecated `status`, plus the season's `type`,
     * `number`, `status` (the viewer's LoadedSequenceItem without the heavy fields). No file: empty title/thumbnail/release.
     */
    private static function summary(mixed $chapter, string $type, mixed $number, mixed $status): stdClass
    {
        if ($chapter instanceof stdClass) {
            $summary = clone $chapter;
            unset($summary->groups, $summary->audio, $summary->status);
        } else {
            $summary = new stdClass();
            $summary->title = '';
            $summary->thumbnail = '';
            $summary->release = '';
        }
        $summary->type = $type;
        $summary->number = $number;
        $summary->status = $status;
        return $summary;
    }

    /** The season status of a chapter (first season whose sequence lists it; the asked locale's season), or null */
    private function chapterStatus(Context $ctx, int $number, string $locale): ?string
    {
        foreach ($this->seasonNumbers($ctx) as $season) {
            $doc = $ctx->pick("season:$ctx->toon:$season", $locale)[1] ?? null;
            foreach ($doc instanceof stdClass && is_array($doc->sequence ?? null) ? $doc->sequence : [] as $item) {
                if (is_object($item) && ($item->type ?? 'chapter') === 'chapter' && (int) ($item->number ?? 0) === $number) {
                    return is_string($item->status ?? null) ? $item->status : self::PUBLISHED;
                }
            }
        }
        return null;
    }

    /** A copy of the chapter with every `ref` group replaced by its scene file (served locale first); missing ones stay */
    private function withScenes(Context $ctx, stdClass $chapter, string $locale): stdClass
    {
        $chapter = clone $chapter;
        $groups = [];
        foreach (is_array($chapter->groups ?? null) ? $chapter->groups : [] as $group) {
            $src = is_object($group) && isset($group->ref) && is_object($group->ref) ? ($group->ref->src ?? null) : null;
            if (!is_string($src)) {
                $groups[] = $group;
                continue;
            }
            $entity = EntityKey::sceneOf($ctx->toon, $src);
            $scene = $entity !== null ? $ctx->pick($entity, $locale) : null;
            if ($scene === null) {
                $ctx->missing[] = $entity ?? $src;
                $groups[] = $group;
                continue;
            }
            if ($scene[0] !== $locale) $ctx->fallback = true;
            $groups[] = $scene[1];
        }
        if (isset($chapter->groups)) $chapter->groups = $groups;
        return $chapter;
    }

    /**
     * Character ids a chapter references (`character` fields anywhere), legacy ids mapped
     * @param array<string, true> $ids
     * @return array<string, true>
     */
    private static function referencedCharacters(mixed $node, array &$ids = []): array
    {
        if (is_object($node)) {
            foreach (get_object_vars($node) as $key => $value) {
                if ($key === 'character' && is_string($value)) $ids[self::LEGACY_CHARACTER_IDS[$value] ?? $value] = true;
                else self::referencedCharacters($value, $ids);
            }
        } elseif (is_array($node)) {
            foreach ($node as $value) self::referencedCharacters($value, $ids);
        }
        return $ids;
    }

    /** The toon's star: the registry character whose `own_toon` is the toon (e.g. catpope → catpope), or null. */
    private static function star(mixed $registry, string $toon): ?string
    {
        foreach (is_object($registry) && is_array($registry->characters ?? null) ? $registry->characters : [] as $c) {
            if (is_object($c) && ($c->own_toon ?? null) === $toon && is_string($c->id ?? null)) return $c->id;
        }
        return null;
    }

    /**
     * The cast with the star in every season (owner rule, agents/ITEMS.md "Cast & featuring"; the creator saves it that
     * way, this covers documents saved before): the star is added in front of a season that lacks it. A copy – the
     * snapshot's document is not changed.
     */
    private static function withStar(mixed $cast, ?string $star): mixed
    {
        if ($star === null || !$cast instanceof stdClass || !is_object($cast->seasons ?? null)) return $cast;
        $out = clone $cast;
        $out->seasons = clone $cast->seasons;
        foreach (get_object_vars($out->seasons) as $season => $entries) {
            if (!is_array($entries)) continue;
            foreach ($entries as $entry) {
                if (is_object($entry) && ($entry->character ?? null) === $star) continue 2;
            }
            $entry = new stdClass();
            $entry->character = $star;
            $entry->personas = ['regular'];
            $out->seasons->{$season} = [$entry, ...$entries];
        }
        return $out;
    }

    /**
     * The people the credits reference (`person` fields), following `merged_into`, in registry order
     * @param array<mixed> $people
     * @return list<mixed>
     */
    private static function referencedPeople(stdClass $credits, array $people): array
    {
        $byId = [];
        foreach ($people as $person) {
            if (is_object($person) && isset($person->id)) $byId[(string) $person->id] = $person;
        }
        $wanted = [];
        $walk = function (mixed $node) use (&$walk, &$wanted): void {
            if (is_object($node)) {
                foreach (get_object_vars($node) as $key => $value) {
                    if ($key === 'person' && is_string($value)) $wanted[$value] = true;
                    else $walk($value);
                }
            } elseif (is_array($node)) {
                foreach ($node as $value) $walk($value);
            }
        };
        $walk($credits);
        foreach (array_keys($wanted) as $id) {
            $seen = [];
            $person = $byId[(string) $id] ?? null;
            while ($person !== null && is_string($person->merged_into ?? null) && !isset($seen[$person->merged_into])) {
                $seen[$person->merged_into] = true;
                $wanted[$person->merged_into] = true;
                $person = $byId[$person->merged_into] ?? null;
            }
        }
        return array_values(array_filter($people, fn($p) => is_object($p) && isset($p->id) && isset($wanted[(string) $p->id])));
    }

    // ── Answer + cache ────────────────────────────────────────────────────────────────────────────────────

    /** @param callable(): array{0: mixed, 1: ?string} $compose [data, served locale] */
    private function answer(string $what, Snapshot $snapshot, callable $compose, ?Context $ctx = null): Answer
    {
        $key = 'library:' . self::FORMAT . ':' . $what . ':' . ($this->showDrafts() ? 'd:' : '') . $snapshot->fingerprint();
        $etag = '"l' . substr(sha1($key), 0, 24) . '"';
        if (!$snapshot->draft) {
            $raw = $this->cache->get($key);
            $cached = $raw === null ? null : Answer::fromCache($raw, $etag);
            if ($cached !== null) return $cached;
        }
        [$data, $locale] = $compose();
        $body = json_encode($data, self::JSON_FLAGS);
        if ($body === false) throw new ApiError(500, 'server_error', 'The answer could not be encoded.');
        $answer = new Answer($body, $etag, $snapshot->draft, $locale, $ctx !== null && $ctx->fallback, array_values(array_unique($ctx === null ? [] : $ctx->missing)));
        if (!$snapshot->draft) $this->cache->set($key, (string) json_encode($answer->toCache(), self::JSON_FLAGS), self::CACHE_TTL);
        return $answer;
    }
}
