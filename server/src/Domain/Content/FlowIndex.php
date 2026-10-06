<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Database\Database;

/**
 * The SCUTOONS-wide flow index (owner 2026-10-06, the creator's universe Flow page, agents/DESIGN.md "Flow panel"): the
 * newest version of every chapter / scene / season / toon document, scanned once for the ways out of a chapter – jumpTo
 * events, choices that jump, a group's authored end (in group scripts, slice triggers, placed items' taps) – plus every
 * toon's seasons and its dead things (the same rules as @scutoons/content `deadThings`: variables set but never read,
 * read but never set, script points and group labels nothing jumps to). The browser draws it without loading every toon.
 * Cached under the version ids it was built from (content-addressed like the library: nothing to invalidate).
 */
final class FlowIndex
{
    private const CACHE_TTL = 3600;
    private const VERSION = 'v1';
    private const KINDS = ['chapter', 'scene', 'season', 'toon', 'map'];
    private const TARGET_TYPES = ['chapter', 'season', 'toon', 'product', 'xr'];
    private const TEXT_VARIABLE = '/\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?::\s*clock\s*)?\}/';

    public function __construct(private readonly Database $db, private readonly Cache $cache)
    {
    }

    /** @return array<string, mixed> */
    public function build(): array
    {
        $placeholders = implode(',', array_fill(0, count(self::KINDS), '?'));
        $latest = $this->db->all("SELECT MAX(id) AS id FROM entity_versions WHERE kind IN ($placeholders) GROUP BY entity_key ORDER BY id", self::KINDS);
        $ids = array_map(fn(array $r) => (int) $r['id'], $latest);
        $key = 'flow-index:' . self::VERSION . ':' . sha1(implode(',', $ids));
        $cached = $this->cache->get($key);
        if ($cached !== null) {
            $decoded = json_decode($cached, true);
            if (is_array($decoded)) return $decoded;
        }
        $answer = $this->compose($this->documents($ids));
        $this->cache->set($key, (string) json_encode($answer, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), self::CACHE_TTL);
        return $answer;
    }

    /**
     * entity (key without locale) → its document: the language-neutral one, else a legacy per-locale one (the first
     * locale – jumps are the same in every language)
     * @param list<int> $ids
     * @return array<string, array<mixed>>
     */
    private function documents(array $ids): array
    {
        $docs = [];
        $legacy = [];
        foreach (array_chunk($ids, 500) as $chunk) {
            if ($chunk === []) continue;
            $rows = $this->db->all('SELECT entity_key, locale, json FROM entity_versions WHERE id IN (' . implode(',', array_fill(0, count($chunk), '?')) . ')', $chunk);
            foreach ($rows as $row) {
                $data = json_decode((string) $row['json'], true);
                if (!is_array($data)) continue;
                $entityKey = (string) $row['entity_key'];
                if ($row['locale'] === null) {
                    $docs[$entityKey] = $data;
                    continue;
                }
                $entity = substr($entityKey, 0, (int) strrpos($entityKey, ':'));
                $locale = (string) $row['locale'];
                if (!isset($legacy[$entity]) || strcmp($locale, $legacy[$entity][0]) < 0) $legacy[$entity] = [$locale, $data];
            }
        }
        foreach ($legacy as $entity => [, $data]) {
            if (!isset($docs[$entity])) $docs[$entity] = $data;
        }
        ksort($docs);
        return $docs;
    }

    /**
     * @param array<string, array<mixed>> $docs
     * @return array<string, mixed>
     */
    private function compose(array $docs): array
    {
        /** @var array<string, array{id: string, title: mixed, status: ?string, seasons: array<int, list<array{number: int, status: mixed}>>}> $toons */
        $toons = [];
        /** @var array<string, array<int, list<mixed>>> $chapters toon → chapter number → resolved groups */
        $chapters = [];
        /** @var array<string, list<array<mixed>>> $toonDocs */
        $toonDocs = [];
        $map = $docs['map'] ?? null;
        foreach ($docs as $entity => $doc) {
            $parts = explode(':', $entity);
            $kind = $parts[0];
            if ($kind === 'map' || count($parts) < 2) continue;
            $toon = $parts[1];
            $toons[$toon] ??= ['id' => $toon, 'title' => null, 'status' => null, 'seasons' => []];
            $toonDocs[$toon][] = $doc;
            if ($kind === 'toon') {
                $toons[$toon]['title'] = $doc['title'] ?? null;
                $toons[$toon]['status'] = is_string($doc['status'] ?? null) ? $doc['status'] : null;
            } elseif ($kind === 'season' && isset($parts[2])) {
                $list = [];
                foreach (is_array($doc['sequence'] ?? null) ? $doc['sequence'] : [] as $item) {
                    if (is_array($item) && ($item['type'] ?? 'chapter') === 'chapter' && is_numeric($item['number'] ?? null)) {
                        $list[] = ['number' => (int) $item['number'], 'status' => $item['status'] ?? null];
                    }
                }
                $toons[$toon]['seasons'][(int) $parts[2]] = $list;
            } elseif ($kind === 'chapter' && isset($parts[2])) {
                $groups = [];
                foreach (is_array($doc['groups'] ?? null) ? $doc['groups'] : [] as $group) {
                    $src = is_array($group) && is_array($group['ref'] ?? null) ? ($group['ref']['src'] ?? null) : null;
                    $scene = is_string($src) ? EntityKey::sceneOf($toon, $src) : null;
                    $groups[] = $scene !== null && isset($docs[$scene]) ? $docs[$scene] : $group;
                }
                $chapters[$toon][(int) $parts[2]] = $groups;
            }
        }
        ksort($toons);
        $edges = [];
        $seen = [];
        $dead = [];
        foreach ($toons as $toon => $entry) {
            $byNumber = $chapters[$toon] ?? [];
            ksort($byNumber);
            foreach ($byNumber as $number => $groups) {
                foreach ($groups as $group) {
                    if (!is_array($group)) continue;
                    $label = is_string($group['label'] ?? null) && $group['label'] !== '' ? $group['label'] : null;
                    foreach ($this->ways($group) as $way) {
                        $edge = ['source' => ['toon' => $toon, 'chapter' => $number, 'group' => $label], 'target' => $way['target'], 'kind' => $way['kind']];
                        if ($way['label'] !== '') $edge['label'] = $way['label'];
                        $id = json_encode($edge);
                        if (isset($seen[$id])) continue;
                        $seen[$id] = true;
                        $edges[] = $edge;
                    }
                }
            }
            $dead[$toon] = $this->dead($byNumber, $this->facts([...($toonDocs[$toon] ?? []), ...($map !== null ? [$map] : [])]));
        }
        $list = [];
        foreach ($toons as $entry) {
            ksort($entry['seasons']);
            $seasons = [];
            foreach ($entry['seasons'] as $n => $items) $seasons[] = ['number' => $n, 'chapters' => $items];
            $list[] = ['id' => $entry['id'], 'title' => $entry['title'], 'status' => $entry['status'], 'seasons' => $seasons];
        }
        return ['status' => 'success', 'toons' => $list, 'edges' => $edges, 'dead' => $dead];
    }

    /**
     * The ways out of a group: jumpTo events and choices that jump (script, slice triggers, item taps), its authored end
     * @param array<mixed> $group
     * @return list<array{target: array<string, mixed>, kind: string, label: string}>
     */
    private function ways(array $group): array
    {
        $out = [];
        $lists = [is_array($group['events'] ?? null) ? $group['events'] : []];
        foreach (is_array($group['slices'] ?? null) ? $group['slices'] : [] as $slice) {
            if (!is_array($slice)) continue;
            $lists[] = is_array($slice['events'] ?? null) ? $slice['events'] : [];
            foreach (is_array($slice['items'] ?? null) ? $slice['items'] : [] as $item) {
                if (is_array($item) && is_array($item['events'] ?? null)) $lists[] = $item['events'];
            }
        }
        foreach ($lists as $events) {
            foreach ($events as $event) {
                if (!is_array($event)) continue;
                $params = is_array($event['params'] ?? null) ? $event['params'] : [];
                if (($event['type'] ?? null) === 'jumpTo' && ($target = self::target($params['target'] ?? null)) !== null) {
                    $out[] = ['target' => $target, 'kind' => 'jump', 'label' => ''];
                }
                if (($event['type'] ?? null) === 'addChoice') {
                    $actions = $params['onSelect'] ?? [];
                    if (is_array($actions) && !array_is_list($actions)) $actions = [$actions];
                    foreach (is_array($actions) ? $actions : [] as $action) {
                        if (is_array($action) && is_array($action['jumpTo'] ?? null) && ($target = self::target($action['jumpTo']['target'] ?? null)) !== null) {
                            $out[] = ['target' => $target, 'kind' => 'choice', 'label' => self::text($params['label'] ?? '')];
                        }
                    }
                }
            }
        }
        if (is_array($group['end'] ?? null) && ($target = self::target($group['end']['target'] ?? null)) !== null) {
            $out[] = ['target' => $target, 'kind' => 'end', 'label' => ''];
        }
        return $out;
    }

    /** @return array<string, mixed>|null a jump target { type, value, label?, anyOf? } when it is one */
    private static function target(mixed $t): ?array
    {
        if (!is_array($t) || !in_array($t['type'] ?? null, self::TARGET_TYPES, true)) return null;
        $value = $t['value'] ?? null;
        if (!is_scalar($value) || (string) $value === '') return null;
        $out = ['type' => $t['type'], 'value' => (string) $value];
        if (is_string($t['label'] ?? null) && $t['label'] !== '') $out['label'] = $t['label'];
        if (is_array($t['anyOf'] ?? null)) {
            $anyOf = array_values(array_filter($t['anyOf'], fn($l) => is_string($l) && $l !== ''));
            if ($anyOf !== []) $out['anyOf'] = $anyOf;
        }
        return $out;
    }

    /** a text as shown: the string, or a locale map's first text */
    private static function text(mixed $value): string
    {
        if (is_string($value)) return $value;
        if (is_array($value)) {
            foreach ($value as $text) if (is_string($text) && trim($text) !== '') return $text;
        }
        return '';
    }

    /**
     * What the toon's documents tell (@scutoons/content `toonFacts`): variables something sets, labels something jumps to
     * @param list<array<mixed>> $docs
     * @return array{variables: array<string, true>, labels: array<string, true>}
     */
    private function facts(array $docs): array
    {
        $variables = [];
        $labels = [];
        $stack = $docs;
        while ($stack !== []) {
            $v = array_pop($stack);
            if (!is_array($v)) continue;
            if (!array_is_list($v)) {
                $params = is_array($v['params'] ?? null) ? $v['params'] : [];
                if (($v['type'] ?? null) === 'setVariable' && is_string($params['variable'] ?? null)) $variables[$params['variable']] = true;
                foreach (['set', 'add'] as $op) {
                    if (is_array($v[$op] ?? null) && is_string($v[$op]['variable'] ?? null)) $variables[$v[$op]['variable']] = true;
                }
                if (($v['action'] ?? null) === 'pickup' && is_string($v['item'] ?? null)) $variables[$v['item']] = true;
                if (is_string($v['type'] ?? null) && array_key_exists('value', $v)) {
                    if (is_string($v['label'] ?? null) && $v['label'] !== '') $labels[$v['label']] = true;
                    foreach (is_array($v['anyOf'] ?? null) ? $v['anyOf'] : [] as $l) if (is_string($l) && $l !== '') $labels[$l] = true;
                }
            }
            foreach ($v as $child) if (is_array($child)) $stack[] = $child;
        }
        return ['variables' => $variables, 'labels' => $labels];
    }

    /**
     * The toon's dead things (@scutoons/content `deadThings`, the same rules)
     * @param array<int, list<mixed>> $chapters
     * @param array{variables: array<string, true>, labels: array<string, true>} $facts
     * @return array<string, list<array{name: string, at: list<array<string, mixed>>}>>
     */
    private function dead(array $chapters, array $facts): array
    {
        $sets = [];
        $reads = [];
        $points = [];
        $labels = [];
        $note = function (array &$map, string $name, array $at): void {
            if ($name === '' || str_starts_with($name, 'picked:')) return;
            foreach ($map[$name] ?? [] as $p) if ($p['chapter'] === $at['chapter'] && $p['group'] === $at['group']) return;
            $map[$name][] = $at;
        };
        foreach ($chapters as $number => $groups) {
            foreach ($groups as $gi => $group) {
                if (!is_array($group)) continue;
                $label = is_string($group['label'] ?? null) && $group['label'] !== '' ? $group['label'] : null;
                $at = ['chapter' => $number, 'group' => $gi] + ($label !== null ? ['label' => $label] : []);
                $named = [];
                $diverted = [];
                $stack = [$group];
                while ($stack !== []) {
                    $v = array_pop($stack);
                    if (is_string($v)) {
                        if (preg_match_all(self::TEXT_VARIABLE, $v, $m)) foreach ($m[1] as $name) $note($reads, $name, $at);
                        continue;
                    }
                    if (!is_array($v)) continue;
                    if (!array_is_list($v)) {
                        $params = is_array($v['params'] ?? null) ? $v['params'] : [];
                        $type = $v['type'] ?? null;
                        if ($type === 'setVariable' && is_string($params['variable'] ?? null)) $note($sets, $params['variable'], $at);
                        foreach (['set', 'add'] as $op) {
                            if (is_array($v[$op] ?? null) && is_string($v[$op]['variable'] ?? null)) $note($sets, $v[$op]['variable'], $at);
                        }
                        if (is_array($v['if'] ?? null) && is_string($v['if']['variable'] ?? null)) $note($reads, $v['if']['variable'], $at);
                        if ($type === 'label' && is_string($params['name'] ?? null) && $params['name'] !== '') $named[] = $params['name'];
                        if ($type === 'divert' && is_string($params['to'] ?? null)) $diverted[$params['to']] = true;
                        if (is_array($v['divert'] ?? null) && is_string($v['divert']['to'] ?? null)) $diverted[$v['divert']['to']] = true;
                    }
                    foreach ($v as $k => $child) if ($k !== 'if') $stack[] = $child;
                }
                foreach (array_unique($named) as $name) if (!isset($diverted[$name])) $points[] = ['name' => $name, 'at' => [$at]];
                // a group label: something refers to it, it starts the chapter, or the follow-up bar leads there
                if ($label !== null && $gi > 0 && !isset($facts['labels'][$label])) {
                    $before = $groups[$gi - 1] ?? null;
                    $bar = is_array($before) && empty($before['continues']) && !self::hasEnd($before);
                    if (!$bar) $labels[] = ['name' => $label, 'at' => [$at]];
                }
            }
        }
        $list = function (array $map, callable $keep): array {
            $out = [];
            foreach ($map as $name => $at) if ($keep((string) $name)) $out[] = ['name' => (string) $name, 'at' => $at];
            usort($out, fn($a, $b) => strcmp($a['name'], $b['name']));
            return $out;
        };
        return [
            'unread' => $list($sets, fn(string $n) => !isset($reads[$n])),
            'unset' => $list($reads, fn(string $n) => !isset($sets[$n]) && !isset($facts['variables'][$n])),
            'points' => $points,
            'labels' => $labels,
        ];
    }

    /** @param array<mixed> $group a valid authored end (`groupEnd` in chains.ts) */
    private static function hasEnd(array $group): bool
    {
        return is_array($group['end'] ?? null) && self::target($group['end']['target'] ?? null) !== null;
    }
}
