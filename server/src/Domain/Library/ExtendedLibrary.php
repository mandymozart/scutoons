<?php
declare(strict_types=1);

namespace Scutoons\Domain\Library;

use Scutoons\Domain\ApiError;
use Scutoons\Generated\LibraryTypes;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Clock\Clock;

/**
 * The extended Library's answers (#864, docs/API.md "The extended Library"; the shapes: server/schema/api/library.schema.json):
 * a place's page of things and files, or a thing stepped into – for one member's rights. Read from UniverseIndex; the
 * answer's version (the ETag) is the index's fingerprint, the rights, the query and the locale, so a 304 needs no answer
 * built, and an answer is built once per version.
 *
 * @phpstan-import-type Index from UniverseIndex
 * @phpstan-import-type Thing from UniverseIndex
 * @phpstan-import-type File from UniverseIndex
 * @phpstan-import-type LibraryEntry from LibraryTypes
 * @phpstan-import-type LibraryLink from LibraryTypes
 * @phpstan-import-type LinkRole from LibraryTypes
 * @phpstan-type Query array{toon: ?string, at: ?string, place: string, type: list<string>, subtype: ?string, kind: list<string>, from: ?string, locale: string, cursor: ?string, limit: int}
 */
final class ExtendedLibrary
{
    /** a page's entries: the default and the most */
    public const LIMIT = 1000;
    public const MAX_LIMIT = 5000;
    /** Recently added: changed in the last days (the window of the shelf's `recent`, deck-catalog library/shelf.rs RECENT_DAYS) */
    public const RECENT_DAYS = 7;
    private const CACHE_TTL = 86400;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_THROW_ON_ERROR;
    private const TOON = '/^[a-z0-9][a-z0-9_-]{0,63}$/';
    private const ADDRESS = '/^[a-z][a-z-]*:[^\s]{1,600}$/';

    /** @var Index|null */
    private ?array $index = null;
    private bool $editor = false;
    private bool $live = false;

    public function __construct(private readonly UniverseIndex $universe, private readonly Cache $cache, private readonly Clock $clock)
    {
    }

    /**
     * The query read and checked (400 on what can never answer)
     * @param array<mixed> $params
     * @return Query
     */
    public static function query(array $params): array
    {
        $str = fn(string $name): ?string => is_string($params[$name] ?? null) && trim($params[$name]) !== '' ? trim($params[$name]) : null;
        $list = fn(string $name): array => array_values(array_unique(array_filter(array_map('trim', explode(',', $str($name) ?? '')), fn(string $s) => $s !== '')));
        $toon = $str('toon');
        if ($toon !== null && preg_match(self::TOON, $toon) !== 1) throw new ApiError(400, 'invalid_toon', 'Not a toon id.');
        $at = $str('at');
        if ($at !== null && preg_match(self::ADDRESS, $at) !== 1) throw new ApiError(400, 'invalid_address', 'Not an address (kind:id).');
        $place = $str('place') ?? ($toon !== null ? 'toon' : 'all');
        $ok = in_array($place, ['toon', 'all', 'recent', 'unused'], true) || preg_match('/^cast:[0-9]{1,6}$/', $place) === 1
            || (str_starts_with($place, 'item:') && preg_match(self::ADDRESS, substr($place, 5)) === 1);
        if (!$ok) throw new ApiError(400, 'invalid_place', 'place is toon | all | recent | unused | item:<address> | cast:<season>.');
        if (($place === 'toon' || str_starts_with($place, 'cast:')) && $toon === null) throw new ApiError(400, 'invalid_place', "The place $place needs ?toon=.");
        $type = $list('type');
        foreach ($type as $t) if (!in_array($t, LibraryTypes::THING_TYPE, true)) throw new ApiError(400, 'invalid_type', "Unknown type $t.", ['types' => LibraryTypes::THING_TYPE]);
        $subtype = $str('subtype');
        if ($subtype !== null && !in_array($subtype, LibraryTypes::ITEM_SUBTYPE, true)) throw new ApiError(400, 'invalid_subtype', "Unknown subtype $subtype.", ['subtypes' => LibraryTypes::ITEM_SUBTYPE]);
        $kind = $list('kind');
        foreach ($kind as $k) if (!in_array($k, LibraryTypes::FILE_KIND, true)) throw new ApiError(400, 'invalid_kind', "Unknown kind $k.", ['kinds' => LibraryTypes::FILE_KIND]);
        $from = $str('from');
        if ($from !== null && !in_array($from, LibraryTypes::FILE_ORIGIN, true)) throw new ApiError(400, 'invalid_from', 'from is uploaded | imported | generated.');
        $locale = $str('locale') ?? 'en';
        if (preg_match('/^[a-z]{2}$/', $locale) !== 1) throw new ApiError(400, 'invalid_locale', 'locale is a two-letter code.');
        $cursor = $str('cursor');
        if ($cursor !== null && self::fromCursor($cursor) === null) throw new ApiError(400, 'invalid_cursor', 'Not a cursor of this list.');
        $limit = $str('limit') ?? (string) self::LIMIT;
        if (!ctype_digit($limit) || (int) $limit < 1 || (int) $limit > self::MAX_LIMIT) throw new ApiError(400, 'invalid_limit', 'limit is 1 to ' . self::MAX_LIMIT . '.');
        sort($type);
        sort($kind);
        return ['toon' => $toon, 'at' => $at, 'place' => $place, 'type' => $type, 'subtype' => $subtype, 'kind' => $kind, 'from' => $from, 'locale' => $locale, 'cursor' => $cursor, 'limit' => (int) $limit];
    }

    /**
     * The answer's version (without quotes) for this member's rights and query, and the answer itself once asked for
     * @param Query $query
     * @return array{version: string, body: callable(): string}
     */
    public function answer(array $query, bool $editor, bool $live): array
    {
        $this->editor = $editor;
        $this->live = $live;
        $fingerprint = $this->universe->fingerprint();
        // the recent place moves with the clock: its version holds the hour
        $hour = $query['place'] === 'recent' ? gmdate('Y-m-d H', $this->clock->time()) : '';
        $version = 'x' . substr(sha1(json_encode([LibraryTypes::FORMAT, $fingerprint, $editor, $live, $query, $hour], self::JSON_FLAGS)), 0, 23);
        $body = function () use ($fingerprint, $query, $version): string {
            $key = "library-answer:$version";
            $cached = $this->cache->get($key);
            if ($cached !== null) return $cached;
            $this->index = $this->universe->index($fingerprint);
            $data = $query['at'] !== null ? $this->step($query['at'], $query['locale'], $version) : $this->page($query, $version);
            $json = json_encode($data, self::JSON_FLAGS);
            $this->cache->set($key, $json, self::CACHE_TTL);
            return $json;
        };
        return ['version' => $version, 'body' => $body];
    }

    // ── A place's page ────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param Query $q
     * @return array<string, mixed> LibraryList
     */
    private function page(array $q, string $version): array
    {
        $index = $this->idx();
        $toon = $q['toon'];
        if ($toon !== null && !isset($index['things']["toon:$toon"])) throw new ApiError(404, 'unknown_toon', "No toon $toon.");
        [$things, $files] = $this->place($q['place'], $toon);
        if ($q['from'] !== null) {
            $things = [];
            $files = array_filter($files, fn(string $a) => $index['files'][$a]['origin'] === $q['from']);
        }
        $counts = [];
        foreach ($things as $a) $counts[$index['things'][$a]['type']] = ($counts[$index['things'][$a]['type']] ?? 0) + 1;
        foreach ($files as $a) $counts[$index['files'][$a]['kind']] = ($counts[$index['files'][$a]['kind']] ?? 0) + 1;
        if ($q['type'] !== [] || $q['kind'] !== [] || $q['subtype'] !== null) {
            $things = $q['type'] === [] && $q['subtype'] === null ? [] : array_filter($things, function (string $a) use ($index, $q): bool {
                $t = $index['things'][$a];
                return ($q['type'] === [] || in_array($t['type'], $q['type'], true)) && ($q['subtype'] === null || ($t['subtype'] ?? null) === $q['subtype']);
            });
            $files = $q['kind'] === [] ? [] : array_filter($files, fn(string $a) => in_array($index['files'][$a]['kind'], $q['kind'], true));
        }
        $keyed = [];
        foreach ($things as $a) $keyed[self::sortKey(array_search($index['things'][$a]['type'], LibraryTypes::THING_TYPE, true), $a)] = $a;
        foreach ($files as $a) $keyed[self::sortKey(100 + (int) array_search($index['files'][$a]['kind'], LibraryTypes::FILE_KIND, true), $a)] = $a;
        ksort($keyed, SORT_STRING);
        $total = count($keyed);
        $after = $q['cursor'] !== null ? self::fromCursor($q['cursor']) : null;
        $page = [];
        $last = null;
        foreach ($keyed as $key => $address) {
            if ($after !== null && strcmp((string) $key, $after) <= 0) continue;
            if (count($page) === $q['limit']) break;
            $page[] = $this->entry($address, $q['locale']);
            $last = (string) $key;
        }
        $more = $last !== null && strcmp($last, (string) array_key_last($keyed)) < 0;
        ksort($counts);
        return ['library' => LibraryTypes::FORMAT, 'version' => $version, 'toon' => $toon, 'place' => $q['place'], 'locale' => $q['locale'], 'entries' => $page,
            'total' => $total, 'next' => $more ? self::toCursor($last) : null, 'counts' => (object) $counts];
    }

    /**
     * The things and files a place holds, readable by the member
     * @return array{0: list<string>, 1: list<string>}
     */
    private function place(string $place, ?string $toon): array
    {
        $index = $this->idx();
        $things = array_values(array_filter(array_keys($index['things']), fn(string $a) => $this->readsThing($a)));
        $files = $this->editor ? array_map('strval', array_keys($index['files'])) : [];
        if (str_starts_with($place, 'item:')) {
            $at = substr($place, 5);
            if (!$this->readsThing($at) && !$this->readsDocument($at)) throw new ApiError(404, 'not_found', "Nothing at $at.");
            return [$this->usedThings($at), $this->usedFiles([$at])];
        }
        if (str_starts_with($place, 'cast:')) {
            $season = substr($place, 5);
            $cast = $index['casts'][(string) $toon][$season] ?? null;
            if ($cast === null) throw new ApiError(404, 'not_found', "The toon $toon has no cast for season $season.");
            $characters = array_values(array_unique(array_filter($cast, fn(string $a) => $this->readsThing($a))));
            return [$characters, $this->usedFiles($characters)];
        }
        if ($toon !== null && $place !== 'all') {
            $things = array_values(array_filter($things, fn(string $a) => in_array($toon, $index['things'][$a]['toons'], true)));
            $files = array_values(array_filter($files, fn(string $a) => in_array($toon, $index['files'][$a]['toons'], true)));
        }
        if ($place === 'recent') {
            $since = $this->clock->time() - self::RECENT_DAYS * 86400;
            $recent = fn(?string $at) => $at !== null && strtotime($at) >= $since;
            $things = array_values(array_filter($things, fn(string $a) => $recent($index['things'][$a]['updated'])));
            $files = array_values(array_filter($files, fn(string $a) => $recent($index['files'][$a]['updated'])));
        } elseif ($place === 'unused') {
            $things = [];
            $files = array_values(array_filter($files, fn(string $a) => $index['files'][$a]['refs'] === []));
        }
        return [$things, $files];
    }

    // ── A thing stepped into ──────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed> LibraryStep */
    private function step(string $at, string $locale, string $version): array
    {
        $index = $this->idx();
        $isFile = isset($index['files'][$at]) && $this->editor;
        if (!$isFile && !$this->readsThing($at)) throw new ApiError(404, 'not_found', "Nothing at $at.");
        $entry = $this->entry($at, $locale);
        if ($isFile) {
            $links = array_map(fn(string $ref) => $this->link($ref, 'reference', $locale), array_values(array_filter($index['files'][$at]['refs'], fn(string $r) => $this->readsUser($r))));
            return ['library' => LibraryTypes::FORMAT, 'version' => $version, 'locale' => $locale, 'at' => $entry, 'levels' => [], 'files' => [], 'uses' => [], 'usedIn' => $links];
        }
        $thing = $index['things'][$at];
        $source = $this->source($thing['toons']);
        $levels = array_map(fn(array $l): array => ['address' => $l['address'], 'level' => $l['level'], 'name' => UniverseIndex::name($l['labels'], $locale, $source, $l['address']),
            'picture' => $l['picture'], 'parent' => $l['parent']] + (isset($l['mood']) ? ['mood' => $l['mood']] : []), $index['levels'][$at] ?? []);
        $usedIn = [];
        foreach ($index['usedBy'][$at] ?? [] as [$from, $as]) {
            if ($this->readsUser($from)) $usedIn[] = $this->link($from, $as, $locale);
        }
        return [
            'library' => LibraryTypes::FORMAT, 'version' => $version, 'locale' => $locale, 'at' => $entry, 'levels' => $levels,
            'files' => array_map(fn(string $a) => $this->entry($a, $locale), $this->usedFiles([$at])),
            'uses' => array_map(fn(string $a) => $this->entry($a, $locale), $this->usedThings($at)),
            'usedIn' => $usedIn,
        ];
    }

    /** @return list<string> the readable things a thing uses */
    private function usedThings(string $at): array
    {
        $out = [];
        foreach ($this->idx()['uses'][$at] ?? [] as [$to]) if ($this->readsThing($to)) $out[] = $to;
        return $out;
    }

    /**
     * @param list<string> $users
     * @return list<string> the files these documents use (content editors only)
     */
    private function usedFiles(array $users): array
    {
        if (!$this->editor) return [];
        $out = [];
        foreach ($users as $u) foreach ($this->idx()['fileUses'][$u] ?? [] as $f) $out[$f] = true;
        $out = array_map('strval', array_keys($out));
        sort($out);
        return $out;
    }

    /**
     * @param LinkRole $as
     * @return LibraryLink
     */
    private function link(string $address, string $as, string $locale): array
    {
        $index = $this->idx();
        $thing = $index['things'][$address] ?? null;
        if ($thing !== null) {
            return ['address' => $address, 'type' => $thing['type'], 'name' => UniverseIndex::name($thing['labels'], $locale, $this->source($thing['toons']), $thing['id']), 'as' => $as];
        }
        $toon = explode(':', $address)[1] ?? null;
        $labels = $index['docs'][$address] ?? [];
        return ['address' => $address, 'name' => UniverseIndex::name($labels, $locale, $toon !== null ? $index['sources'][$toon] ?? null : null, $address), 'as' => $as];
    }

    // ── Entries and rights ────────────────────────────────────────────────────────────────────────────────

    /** @return LibraryEntry */
    private function entry(string $address, string $locale): array
    {
        $index = $this->idx();
        $thing = $index['things'][$address] ?? null;
        if ($thing !== null) {
            $usedIn = count(array_filter($index['usedBy'][$address] ?? [], fn(array $e) => $this->readsUser($e[0])));
            return ['address' => $address, 'type' => $thing['type']] + (isset($thing['subtype']) ? ['subtype' => $thing['subtype']] : [])
                + ['name' => UniverseIndex::name($thing['labels'], $locale, $this->source($thing['toons']), $thing['id']), 'picture' => $thing['picture']]
                + (isset($thing['parent']) ? ['parent' => $thing['parent']] : []) + ['toons' => $thing['toons'], 'usedIn' => $usedIn, 'updated' => $thing['updated']];
        }
        $file = $index['files'][$address];
        $usedIn = count(array_filter($file['refs'], fn(string $r) => $this->readsUser($r)));
        return ['address' => $address, 'kind' => $file['kind'], 'name' => $file['name'], 'picture' => $file['picture'], 'toons' => $file['toons'], 'usedIn' => $usedIn, 'updated' => $file['updated'], 'origin' => $file['origin']];
    }

    /** whether the member reads this thing as reading it today: the team (content editors, `live`), shows `live`, the rest content editors */
    private function readsThing(string $address): bool
    {
        $thing = $this->idx()['things'][$address] ?? null;
        if ($thing === null) return false;
        return match ($thing['read']) {
            UniverseIndex::LIVE => $this->live,
            UniverseIndex::EDITOR => $this->editor,
            default => $this->editor || $this->live,
        };
    }

    /** whether the member reads a chapter or a season of a toon – no entry of its own, but a place (This item) */
    private function readsDocument(string $address): bool
    {
        return ($this->editor || $this->live) && preg_match('/^(chapter|season):/', $address) === 1 && isset($this->idx()['docs'][$address]);
    }

    /** whether a user of something counts for the member: a thing they read, or a document of a toon (a season, a chapter, a cast …) */
    private function readsUser(string $address): bool
    {
        if (isset($this->idx()['things'][$address])) return $this->readsThing($address);
        return !str_starts_with($address, 'show:') || $this->live;
    }

    /** @param list<string> $toons */
    private function source(array $toons): ?string
    {
        return isset($toons[0]) ? $this->idx()['sources'][$toons[0]] ?? null : null;
    }

    /** @return Index */
    private function idx(): array
    {
        return $this->index ?? throw new \LogicException('the index is read in answer()');
    }

    private static function sortKey(int|string|false $rank, string $address): string
    {
        return sprintf('%03d', (int) $rank) . "\x1f" . $address;
    }

    private static function toCursor(string $key): string
    {
        return rtrim(strtr(base64_encode($key), '+/', '-_'), '=');
    }

    private static function fromCursor(string $cursor): ?string
    {
        $key = base64_decode(strtr($cursor, '-_', '+/'), true);
        return is_string($key) && preg_match('/^[0-9]{3}\x1f\S/', $key) === 1 ? $key : null;
    }
}
