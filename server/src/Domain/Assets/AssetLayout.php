<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Infrastructure\AssetStorage\StorageKey;
use Scutoons\Infrastructure\Database\Database;

/**
 * The CDN folder convention = the toon hierarchy (owner 2026-10-02, agents/ASSETS.md "Folder convention"):
 *   scutoons/…                  the universe: items, models, platform-wide media
 *   characters/<id>/…           the character registry (unchanged)
 *   <toon>/…                    toon level: avatar, items, merch, map, characters of the toon
 *   <toon>/s<n>/…               season: covers, trailer, album / music, supporters
 *   <toon>/s<n>/c<n>/…          chapter: slices, key visual, thumbnail, chapter video / audio / samples
 * No kind folders (`images/`, `images/webtoon/s1c2/`, `videos/`, `audio/`) where the season / chapter folder says it;
 * file names keep their ordering prefix (`s1c2-slice-14.webp`). Resolution / density variants and posters are SUFFIXES,
 * never folders: `.480p` `.720p` `.1080p`, `.1280w`, `.2x` `.3x` (1x = no suffix), `.poster`.
 *
 * This class only PLANS: old path → new path for every file of the storage, with the reason and the uses a move would
 * rewrite. The move itself is AssetService::relink / rename (content gets drafts) – timed with the bunny switch.
 */
final class AssetLayout
{
    /** paths built in app code (not in content) – a move needs a code change there too */
    public const CODE_REFERENCES = [
        'models/' => 'viewer/src/lib/components/camera/object-detector.ts (MOBILENET_MODEL)',
    ];

    public function __construct(private readonly Database $db)
    {
    }

    /**
     * toon → chapter number → season number, from the seasons' `sequence` (newest version, any locale)
     * @return array<string, array<int, int>>
     */
    public function chapterSeasons(): array
    {
        $map = [];
        $rows = $this->db->all("SELECT v.entity_key, v.json FROM entity_versions v
            WHERE v.kind = 'season' AND v.id = (SELECT MAX(w.id) FROM entity_versions w WHERE w.entity_key = v.entity_key)");
        foreach ($rows as $row) {
            if (!preg_match('/^season:([a-z0-9_-]+):(\d+):/', (string) $row['entity_key'], $m)) continue;
            $doc = json_decode((string) $row['json'], true);
            foreach ((array) ($doc['sequence'] ?? []) as $entry) {
                if (is_array($entry) && isset($entry['number']) && is_numeric($entry['number'])) $map[$m[1]][(int) $entry['number']] ??= (int) $m[2];
            }
        }
        return $map;
    }

    /**
     * The plan for these storage keys ($extraUses: uses the index doesn't know, e.g. paths in app code)
     * @param list<string> $keys
     * @param array<string, list<array<string, mixed>>> $extraUses
     * @return array{items: list<array<string, mixed>>, summary: array<string, mixed>}
     */
    public function plan(array $keys, array $extraUses = []): array
    {
        $seasons = $this->chapterSeasons();
        $uses = $extraUses;
        foreach ($this->db->all('SELECT path, source, ref, state FROM asset_usages') as $u) $uses[(string) $u['path']][] = $u;
        $items = [];
        $targets = [];
        sort($keys);
        foreach ($keys as $key) {
            if (StorageKey::junk($key)) continue;
            [$new, $reason] = $this->target($key, $uses[$key] ?? [], $seasons);
            $code = null;
            foreach (self::CODE_REFERENCES as $prefix => $where) if (str_starts_with($key, $prefix)) $code = $where;
            $action = $new === '' ? 'delete' : ($new !== $key ? 'move' : 'stay');
            $items[] = ['from' => $key, 'to' => $new === '' ? null : $new, 'action' => $action, 'reason' => $reason, 'moves' => $action === 'move', 'uses' => count($uses[$key] ?? []), 'code' => $code,
                'refs' => array_values(array_unique(array_map(fn(array $u) => $u['ref'] . ' (' . $u['state'] . ')', $uses[$key] ?? [])))];
            if ($new !== '') $targets[$new][] = $key;
        }
        $conflicts = array_filter($targets, fn(array $list) => count($list) > 1);
        foreach ($items as &$item) $item['conflict'] = $item['to'] !== null && isset($conflicts[$item['to']]);
        unset($item);
        $reasons = [];
        foreach ($items as $item) $reasons[$item['reason']] = ($reasons[$item['reason']] ?? 0) + 1;
        ksort($reasons);
        $moving = array_filter($items, fn($i) => $i['moves']);
        return ['items' => $items, 'summary' => [
            'files' => count($items),
            'moves' => count($moving),
            'stay' => count(array_filter($items, fn($i) => $i['action'] === 'stay')),
            'deletes' => count(array_filter($items, fn($i) => $i['action'] === 'delete')),
            'delete_list' => array_values(array_map(fn($i) => $i['from'], array_filter($items, fn($i) => $i['action'] === 'delete'))),
            'uses_to_rewrite' => array_sum(array_map(fn($i) => $i['uses'], $moving)),
            'documents_to_touch' => count(array_unique(array_merge(...array_map(fn($i) => array_map(fn($r) => preg_replace('/ \(.*$/', '', $r), $i['refs']), array_values($moving)) ?: [[]]))),
            'conflicts' => count($conflicts),
            'unplaced' => $reasons['unplaced'] ?? 0,
            'code_changes' => array_values(array_unique(array_filter(array_map(fn($i) => $i['moves'] ? $i['code'] : null, $items)))),
            'by_reason' => $reasons,
        ]];
    }

    /**
     * @param list<array<string, mixed>> $uses
     * @param array<string, array<int, int>> $seasons
     * @return array{0: string, 1: string} new key + reason
     */
    public function target(string $key, array $uses, array $seasons): array
    {
        $parts = explode('/', $key);
        $top = $parts[0];
        if ($top === 'models') return ['scutoons/' . $key, 'universe: models'];
        if ($top === 'scutoons' || $top === 'characters') return [$key, 'unchanged: ' . $top];
        if (count($parts) === 1) return [$key, 'unchanged: root file'];
        $toon = $top;
        $rest = array_slice($parts, 1);
        $name = (string) array_pop($rest);
        // density folders (images/1x|2x|3x[/<locale>]) → a suffix (1x = none); a locale folder is kept as a smell to fix
        $suffix = '';
        $locale = '';
        $dirs = [];
        foreach ($rest as $dir) {
            if (preg_match('/^([123])x$/', $dir, $m)) $suffix = $m[1] === '1' ? '' : '.' . $m[1] . 'x';
            elseif (preg_match('/^[a-z]{2}$/', $dir) && $dir !== 's1') $locale = $dir;
            else $dirs[] = $dir;
        }
        $file = $suffix === '' ? $name : self::withSuffix($name, $suffix);
        $tail = ($locale !== '' ? $locale . '/' : '') . $file;
        if ($rest === [] ) return [$key, 'unchanged: toon level'];

        // the delivered masters with baked-in lettering ("S1C1 - Slice 7.jpg", unused since the text extraction):
        // DELETED with the move (owner 2026-10-02: no masters/ folder) – unless something still uses one
        if (preg_match('/^S(\d+)C(\d+) - Slice (\d+)\.(\w+)$/i', $name) && !$uses) return ['', 'delete: lettered master (owner)'];
        // the name says where it belongs: s1c2-… → season 1 chapter 2; s1-… → season 1
        if (preg_match('/^s(\d+)c(\d+)-/', $name, $m)) return ["$toon/s{$m[1]}/c{$m[2]}/$tail", 'chapter (from the name)'];
        if (preg_match('/^s(\d+)-/', $name, $m)) {
            $topic = in_array('supporters', $dirs, true) ? 'supporters/' : '';
            return ["$toon/s{$m[1]}/$topic$tail", 'season (from the name)'];
        }
        // topic folders of the toon stay topic folders (items, merch, map, the toon's characters)
        foreach (['items', 'merch', 'map', 'characters', 'vectors'] as $topic) {
            $at = array_search($topic, $dirs, true);
            if ($at !== false && $topic === 'vectors' && !$uses) return ['', 'delete: unused, no place (owner)'];
            if ($at !== false) return ["$toon/" . implode('/', array_slice($dirs, (int) $at)) . "/$tail", 'toon topic: ' . $topic];
        }
        // otherwise: the most specific place that uses it (chapter < season < toon)
        $place = $this->placeOf($toon, $uses, $seasons);
        if ($place !== null) {
            $topic = in_array('supporters', $dirs, true) ? 'supporters/' : (in_array('samples', $dirs, true) ? 'samples/' : '');
            return ["$toon/$place/$topic$tail", 'from its uses'];
        }
        // no rule and nothing uses it: DELETED with the move (owner 2026-10-02: collectables, s1s*-thumbnails, vectors)
        if (!$uses) return ['', 'delete: unused, no place (owner)'];
        return ["$toon/" . implode('/', array_values(array_diff($dirs, ['images', 'videos', 'audio', 'webtoon']))) . (array_diff($dirs, ['images', 'videos', 'audio', 'webtoon']) ? '/' : '') . $tail, 'unplaced'];
    }

    /**
     * @param list<array<string, mixed>> $uses
     * @param array<string, array<int, int>> $seasons
     */
    private function placeOf(string $toon, array $uses, array $seasons): ?string
    {
        $chapters = [];
        $seasonNumbers = [];
        foreach ($uses as $u) {
            $ref = (string) $u['ref'];
            if (preg_match('/^(chapter|scene):' . preg_quote($toon, '/') . ':(\d+)/', $ref, $m)) $chapters[(int) $m[2]] = true;
            elseif (preg_match('/^season:' . preg_quote($toon, '/') . ':(\d+)/', $ref, $m)) $seasonNumbers[(int) $m[1]] = true;
            elseif ($u['source'] === 'catalog') $seasonNumbers[1] ??= true; // albums / products: the toon's first season (owner reviews)
        }
        if (count($chapters) === 1 && !$seasonNumbers) {
            $chapter = (int) array_key_first($chapters);
            $season = $seasons[$toon][$chapter] ?? 1;
            return "s$season/c$chapter";
        }
        foreach (array_keys($chapters) as $chapter) $seasonNumbers[$seasons[$toon][$chapter] ?? 1] = true;
        if (count($seasonNumbers) === 1) return 's' . array_key_first($seasonNumbers);
        return null;
    }

    /** "a.jpg" + ".3x" → "a.3x.jpg"; keeps an existing quality marker in front ("a.720p.mp4" + ".3x" → "a.720p.3x.mp4") */
    public static function withSuffix(string $name, string $suffix): string
    {
        $dot = strrpos($name, '.');
        return $dot === false ? $name . $suffix : substr($name, 0, $dot) . $suffix . substr($name, $dot);
    }
}
