<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * The reader's entity service (agents/DATA.md §3, docs/API.md "Library"): toons, seasons, chapters with their scenes
 * inlined, cast, credits and events, composed from the content store's documents (`content_versions`, keyed by
 * scope/locale/path – the configuration format of @scutoons/content stays the format of every document).
 *
 * - Locale: the asked one, else the toon's source language, else another language of the toon (per document).
 * - Visibility (the viewer's rules, now on the server): toon index entry + toon.json must be public, a chapter's
 *   status is the season's `sequence[].status`; `coming-soon` → 403 coming_soon, `draft` / not listed → 403 not_public.
 *   CONTENT_SHOW_DRAFTS makes `draft` readable (local development). Draft mode (creators) reads the newest versions and
 *   skips every gate.
 * - Cache: answers are cached under endpoint + snapshot fingerprint (all version ids of the scopes), so a publish
 *   changes the key – no invalidation. Drafts are never cached.
 */
final class Library
{
    /** The platform pseudo-toon of global documents (characters, people, roles, events) = CREDITS_SCOPE */
    public const PLATFORM = 'scutoons';
    public const NEUTRAL = ContentService::NEUTRAL_LOCALE;
    public const PUBLISHED = 'published';
    public const DRAFT = 'draft';
    public const COMING_SOON = 'coming-soon';
    public const DEFAULT_SOURCE = 'de';
    public const LOCALE_PATTERN = '#^[a-z]{2}$#';
    /** Bump when the shape of an answer changes: old cache entries are never read again */
    private const FORMAT = 1;
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

    /** GET /library/toons → ToonIndex[] (published entries; drafts too with CONTENT_SHOW_DRAFTS; all in draft mode) */
    public function toons(bool $draft): Answer
    {
        $snapshot = Snapshot::toonIndex($this->db, $draft);
        return $this->answer('toons', $snapshot, function () use ($snapshot, $draft): array {
            $list = [];
            foreach ($snapshot->scopes() as $toon) {
                if ($toon === self::PLATFORM) continue;
                $entry = $snapshot->doc($toon, self::NEUTRAL, 'index.json');
                if (!$entry instanceof stdClass) continue;
                $status = $entry->status ?? null;
                $visible = $draft || $status === self::PUBLISHED || ($status === self::DRAFT && $this->showDrafts());
                if ($visible) $list[] = $entry;
            }
            return [$list, null];
        });
    }

    /** GET /library/toons/{toon}?locale= → { entry, toon, seasons: SeasonOverview[], locations, routes, collectables } */
    public function toon(string $toon, ?string $locale, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        $locale ??= $ctx->source;
        return $this->answer("toon:$toon:$locale", $ctx->snapshot, function () use ($ctx, $locale): array {
            [$served, $toonDoc] = $this->visibleToon($ctx, $locale);
            $seasons = [];
            foreach ($this->seasonNumbers($ctx) as $number) {
                $overview = $this->seasonOverview($ctx, $number, $locale);
                if ($overview !== null) $seasons[] = $overview;
            }
            $list = fn(string $path) => $ctx->pick($path, $locale)[1] ?? [];
            return [[
                'entry' => $ctx->entry,
                'toon' => $toonDoc,
                'seasons' => $seasons,
                'locations' => $list('locations.json'),
                'routes' => $list('routes.json'),
                'collectables' => $list('collectables.json'),
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
                    $picked = $ctx->pick("chapter$number.json", $locale);
                    throw new ApiError(403, 'coming_soon', "Chapter $number is coming soon.",
                        ['chapter' => self::summary($picked[1] ?? null, 'chapter', $number, $status)]);
                }
                if ($status === null ? !$ctx->includeDrafts : !$ctx->isPublic($status)) {
                    throw new ApiError(403, 'not_public', "Chapter $number is not published.");
                }
            }
            $ctx->fallback = false; // the answer's fallback flag = the chapter + its scenes
            $picked = $ctx->pick("chapter$number.json", $locale)
                ?? throw new ApiError(404, 'not_found', "Chapter $number of $ctx->toon not found.");
            [$served, $chapter] = $picked;
            if (!$chapter instanceof stdClass) throw new ApiError(404, 'not_found', "Chapter $number of $ctx->toon not found.");
            return [$this->withScenes($ctx, $chapter, $served), $served];
        }, $ctx);
    }

    /**
     * GET /library/toons/{toon}/cast[?chapter=N] → { registry, cast }: the character registry reduced to the toon's cast
     * (or to the characters the chapter's events reference) + the toon's cast.json; null where a document is missing.
     * Language-neutral: the client resolves names/images per locale with resolveCast().
     */
    public function cast(string $toon, ?int $chapter, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        return $this->answer("cast:$toon:" . ($chapter ?? '*'), $ctx->snapshot, function () use ($ctx, $chapter): array {
            $this->visibleToon($ctx, $ctx->source);
            $cast = $ctx->snapshot->doc($ctx->toon, self::NEUTRAL, 'cast.json');
            $registry = $ctx->snapshot->doc(self::PLATFORM, self::NEUTRAL, 'characters.json');
            if ($chapter !== null) {
                $doc = $ctx->pick("chapter$chapter.json", $ctx->source);
                if ($doc === null || !$doc[1] instanceof stdClass) throw new ApiError(404, 'not_found', "Chapter $chapter of $ctx->toon not found.");
                $ids = self::referencedCharacters($this->withScenes($ctx, $doc[1], $doc[0]));
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

    /** GET /library/toons/{toon}/credits → { credits, people (referenced only), roles }; 404 without credits.json */
    public function credits(string $toon, bool $draft): Answer
    {
        $ctx = $this->context($toon, $draft);
        return $this->answer("credits:$toon", $ctx->snapshot, function () use ($ctx): array {
            $this->visibleToon($ctx, $ctx->source);
            $credits = $ctx->snapshot->doc($ctx->toon, self::NEUTRAL, 'credits.json');
            if (!$credits instanceof stdClass) throw new ApiError(404, 'not_found', "Toon $ctx->toon has no credits.");
            $people = $ctx->snapshot->doc(self::PLATFORM, self::NEUTRAL, 'people.json');
            if ($people instanceof stdClass && is_array($people->people ?? null)) {
                $people = clone $people;
                $people->people = self::referencedPeople($credits, $people->people);
            } else {
                $people = null;
            }
            $roles = $ctx->snapshot->doc(self::PLATFORM, self::NEUTRAL, 'roles.json');
            $ctx->fallback = false;
            return [['credits' => $credits, 'people' => $people, 'roles' => $roles instanceof stdClass ? $roles : null], null];
        }, $ctx);
    }

    /** GET /events → the stored events document (`scutoons/_/events.json`) */
    public function events(bool $draft): Answer
    {
        $snapshot = Snapshot::load($this->db, [self::PLATFORM], $draft);
        return $this->answer('events', $snapshot, function () use ($snapshot): array {
            $events = $snapshot->doc(self::PLATFORM, self::NEUTRAL, 'events.json')
                ?? throw new ApiError(404, 'not_found', 'No events stored.');
            return [$events, null];
        });
    }

    // ── Composition ───────────────────────────────────────────────────────────────────────────────────────

    private function context(string $toon, bool $draft): Context
    {
        if (!preg_match(ContentService::TOON_PATTERN, $toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        if ($toon === self::PLATFORM) throw new ApiError(404, 'not_found', "Unknown toon $toon.");
        $snapshot = Snapshot::load($this->db, [$toon, self::PLATFORM], $draft);
        $entry = $snapshot->doc($toon, self::NEUTRAL, 'index.json');
        $entry = $entry instanceof stdClass ? $entry : null;
        [$source, $languages] = self::languages($entry, $snapshot->locales($toon));
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
     * The toon's toon.json when the toon is visible (index entry + toon.json public; production needs the index
     * entry), else 404 – like the viewer's toon layout
     * @return array{0: string, 1: stdClass}
     */
    private function visibleToon(Context $ctx, string $locale): array
    {
        $notFound = new ApiError(404, 'not_found', "Unknown toon $ctx->toon.");
        if (!$ctx->draft) {
            if ($ctx->entry === null ? !$ctx->includeDrafts : !$ctx->isPublic($ctx->entry->status ?? null)) throw $notFound;
        }
        $picked = $ctx->pick('toon.json', $locale);
        if ($picked === null || !$picked[1] instanceof stdClass) throw $notFound;
        if (!$ctx->draft && !$ctx->isPublic($picked[1]->status ?? null)) throw $notFound;
        $ctx->servedLocale = $picked[0];
        return [$picked[0], $picked[1]];
    }

    /** @return list<int> season numbers the toon has in any locale */
    private function seasonNumbers(Context $ctx): array
    {
        $numbers = [];
        foreach ($ctx->languages as $locale) {
            foreach ($ctx->snapshot->paths($ctx->toon, $locale) as $path) {
                if (preg_match('#^season(\d+)\.json$#', $path, $m)) $numbers[(int) $m[1]] = true;
            }
        }
        $numbers = array_keys($numbers);
        sort($numbers);
        return $numbers;
    }

    /** SeasonOverview { number, season, chapters: ChapterSummary[] } – the toon layout's chapter list, or null */
    private function seasonOverview(Context $ctx, int $number, string $locale): ?stdClass
    {
        $picked = $ctx->pick("season$number.json", $locale);
        if ($picked === null || !$picked[1] instanceof stdClass) return null;
        [$served, $season] = $picked;
        $chapters = [];
        foreach (is_array($season->sequence ?? null) ? $season->sequence : [] as $item) {
            $itemNumber = is_object($item) ? ($item->number ?? null) : null;
            if (!is_int($itemNumber) && !is_string($itemNumber)) continue;
            $status = $item->status ?? null;
            if (!$ctx->draft && !$ctx->isListed($status)) continue;
            $type = is_string($item->type ?? null) ? $item->type : 'chapter';
            $path = $type . $itemNumber . '.json';
            $chapter = $ctx->pick($path, $served);
            if ($chapter === null) {
                if ($status !== self::COMING_SOON) $ctx->missing[] = $path;
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
            $doc = $ctx->pick("season$season.json", $locale)[1] ?? null;
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
            $scene = preg_match(ContentService::PATH_PATTERN, $src) === 1 ? $ctx->pick($src, $locale) : null;
            if ($scene === null) {
                $ctx->missing[] = $src;
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
