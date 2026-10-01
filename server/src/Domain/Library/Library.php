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
    /** Bump when the shape of an answer changes: old cache entries are never read again */
    private const FORMAT = 2;
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
     * GET /library/toons → ToonIndex[]: published entries + announced ones (toon settings `announce: true`, marked
     * `announced: true` – their pages stay 404, so no season or chapter of them is exposed); drafts too with
     * CONTENT_SHOW_DRAFTS; all in draft mode. The dev-only toons (sandbox) are never listed publicly. Every entry carries
     * the settings' landing `tile` when there is one.
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
                $status = $entry->status ?? null;
                $announced = $status !== self::PUBLISHED && ($settings->announce ?? null) === true;
                $public = $status === self::PUBLISHED || $announced;
                $visible = $draft || ($status === self::DRAFT && $this->showDrafts())
                    || ($public && !in_array($toon, self::DEV_ONLY_TOONS, true));
                if (!$visible) continue;
                $entry = clone $entry;
                if ($announced) $entry->announced = true;
                $tile = $settings->tile ?? null;
                if ($tile instanceof stdClass) $entry->tile = $tile;
                $list[] = $entry;
            }
            return [$list, null];
        });
    }

    /** The toon settings the list reads `announce` and `tile` from (language-neutral): the source language's, else any */
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

    /** GET /library/toons/{toon}?locale= → { entry, toon, seasons: SeasonOverview[], locations, routes } */
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
            $list = fn(string $kind) => $ctx->pick("$kind:$toon", $locale)[1] ?? [];
            return [[
                'entry' => $ctx->entry,
                'toon' => $toonDoc,
                'seasons' => $seasons,
                'locations' => $list('locations'),
                'routes' => $list('routes'),
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
