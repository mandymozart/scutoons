<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\Content\Expr\Analyser;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Database\Database;

/**
 * The SCUTOONS-wide flow index (owner 2026-10-06, the creator's universe Flow page, agents/DESIGN.md "Flow panel"): the
 * newest version of every chapter / scene / season / toon document, scanned once for the ways out of a chapter – jumpTo
 * events, choices that jump, a group's authored end (in group scripts, slice triggers, placed items' taps) – plus every
 * toon's seasons and its dead things (the same rules as @scutoons/content `deadThings`: variables set but never read,
 * read but never set (reads: simple tests, `if.expr` conditions and texts' `{…}` through the expression analyser), script points and group labels nothing jumps to –
 * from this toon or another one: a chapter target with `toon`). Story variables are ONE save game for every toon (owner
 * 2026-10-06, DATA.md "The save game"); names have a domain (`Variables`: `x` = this toon's, `global.x`, `toptoy.x`) –
 * `variables` lists every storage key with the toons that set / read it, and the dead things judge variables across all
 * toons by key (shown as written in the toon). The browser draws it without loading
 * every toon. The Flow's pictures (owner 2026-10-06): each toon's `avatar` and per chapter its thumbnail and the low
 * variant of each labelled group's first slice (`chapters`, the same as @scutoons/content `chapterPictures`).
 * Cached under the version ids it was built from (content-addressed like the library: nothing to invalidate).
 */
final class FlowIndex
{
    private const CACHE_TTL = 3600;
    private const VERSION = 'v6';
    private const KINDS = ['chapter', 'scene', 'season', 'toon', 'map'];
    private const TARGET_TYPES = ['chapter', 'season', 'toon', 'product', 'xr'];

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
        /** @var array<string, array{id: string, title: mixed, status: ?string, avatar: ?string, seasons: array<int, list<array{number: int, status: mixed}>>, chapters: array<int, array<string, mixed>>}> $toons */
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
            $toons[$toon] ??= ['id' => $toon, 'title' => null, 'status' => null, 'avatar' => null, 'seasons' => [], 'chapters' => []];
            $toonDocs[$toon][] = $doc;
            if ($kind === 'toon') {
                $toons[$toon]['title'] = $doc['title'] ?? null;
                $toons[$toon]['status'] = is_string($doc['status'] ?? null) ? $doc['status'] : null;
                $toons[$toon]['avatar'] = is_string($doc['avatar'] ?? null) && $doc['avatar'] !== '' ? $doc['avatar'] : null;
            } elseif ($kind === 'season' && isset($parts[2])) {
                $list = [];
                foreach (is_array($doc['sequence'] ?? null) ? $doc['sequence'] : [] as $item) {
                    if (is_array($item) && is_numeric($item['number'] ?? null)) {
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
                $toons[$toon]['chapters'][(int) $parts[2]] = self::pictures($groups, $doc['thumbnail'] ?? null);
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
        }
        // dead things: a label another toon jumps to (a chapter target with `toon`) is reached from there
        $incoming = [];
        foreach ($edges as $edge) {
            $target = $edge['target'];
            $into = $target['toon'] ?? null;
            if (!is_string($into) || $into === $edge['source']['toon']) continue;
            foreach ([...(isset($target['label']) ? [$target['label']] : []), ...($target['anyOf'] ?? [])] as $l) $incoming[$into][$l] = true;
        }
        // first every toon's facts and reads, then the dead things over ALL toons' variables (one save game)
        $scans = [];
        /** @var array<string, array{set: array<string, true>, read: array<string, true>}> $variables */
        $variables = [];
        foreach ($toons as $toon => $entry) {
            $byNumber = $chapters[$toon] ?? [];
            ksort($byNumber);
            $facts = $this->facts((string) $toon, [...($toonDocs[$toon] ?? []), ...($map !== null ? [$map] : [])]);
            $facts['labels'] += $incoming[$toon] ?? [];
            $scan = $this->scan((string) $toon, $byNumber, $facts);
            $scans[$toon] = $scan;
            foreach (array_keys($facts['variables']) as $name) $variables[(string) $name]['set'][(string) $toon] = true;
            foreach (array_keys($scan['sets']) as $name) $variables[(string) $name]['set'][(string) $toon] = true;
            foreach (array_keys($scan['reads']) as $name) $variables[(string) $name]['read'][(string) $toon] = true;
        }
        foreach ($scans as $toon => $scan) $dead[$toon] = $this->dead((string) $toon, $scan, $variables);
        ksort($variables, SORT_STRING);
        $variableList = [];
        foreach ($variables as $name => $v) {
            $set = array_map('strval', array_keys($v['set'] ?? []));
            $read = array_map('strval', array_keys($v['read'] ?? []));
            sort($set);
            sort($read);
            $variableList[(string) $name] = ['set' => $set, 'read' => $read];
        }
        $list = [];
        foreach ($toons as $entry) {
            ksort($entry['seasons']);
            $seasons = [];
            foreach ($entry['seasons'] as $n => $items) $seasons[] = ['number' => $n, 'chapters' => $items];
            ksort($entry['chapters']);
            $pictures = [];
            foreach ($entry['chapters'] as $n => $p) $pictures[(string) $n] = (object) $p;
            $list[] = ['id' => $entry['id'], 'title' => $entry['title'], 'status' => $entry['status'], 'seasons' => $seasons, 'avatar' => $entry['avatar'], 'chapters' => (object) $pictures];
        }
        return ['status' => 'success', 'toons' => $list, 'edges' => $edges, 'dead' => $dead, 'variables' => (object) $variableList, 'placeUnlocks' => (object) $this->placeUnlocks($docs)];
    }

    /**
     * place id → the documents (entity keys) whose events unlock it – unlockLocation in a script, a slice trigger or a
     * placed item's tap (agents/MAP.md §8.5b): the map builder warns about a hidden place nothing unlocks
     * @param array<string, array<mixed>> $docs
     * @return array<string, list<string>>
     */
    private function placeUnlocks(array $docs): array
    {
        $out = [];
        foreach ($docs as $entity => $doc) {
            $stack = [$doc];
            while ($stack) {
                $v = array_pop($stack);
                if (!is_array($v)) continue;
                $params = $v['params'] ?? null;
                if (($v['type'] ?? null) === 'unlockLocation' && is_array($params) && is_string($params['location'] ?? null) && $params['location'] !== '') {
                    $out[$params['location']][(string) $entity] = true;
                }
                foreach ($v as $child) if (is_array($child)) $stack[] = $child;
            }
        }
        ksort($out);
        return array_map(fn(array $where) => array_keys($where), $out);
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

    /**
     * a jump target { type, value, toon?, label?, anyOf? } when it is one (`toon`: a chapter of another toon)
     * @return array<string, mixed>|null
     */
    private static function target(mixed $t): ?array
    {
        if (!is_array($t) || !in_array($t['type'] ?? null, self::TARGET_TYPES, true)) return null;
        $value = $t['value'] ?? null;
        if (!is_scalar($value) || (string) $value === '') return null;
        $out = ['type' => $t['type'], 'value' => (string) $value];
        if ($t['type'] === 'chapter' && is_string($t['toon'] ?? null) && $t['toon'] !== '') $out['toon'] = $t['toon'];
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
     * (a target into another toon – `toon` – refers to that toon's labels)
     * @param list<array<mixed>> $docs
     * @return array{variables: array<string, true>, labels: array<string, true>}
     */
    private function facts(string $toon, array $docs): array
    {
        $variables = [];
        $labels = [];
        $stack = $docs;
        $add = function (string $written) use (&$variables, $toon): void {
            $variables[Variables::key($written, $toon)] = true;
        };
        while ($stack !== []) {
            $v = array_pop($stack);
            if (!is_array($v)) continue;
            if (!array_is_list($v)) {
                $params = is_array($v['params'] ?? null) ? $v['params'] : [];
                if (($v['type'] ?? null) === 'setVariable' && is_string($params['variable'] ?? null)) $add($params['variable']);
                if (($asked = self::askVariable($v)) !== null) $add($asked);
                foreach (['set', 'add'] as $op) {
                    if (is_array($v[$op] ?? null) && is_string($v[$op]['variable'] ?? null)) $add($v[$op]['variable']);
                }
                $other = is_string($v['toon'] ?? null) && $v['toon'] !== '' && $v['toon'] !== $toon;
                if (is_string($v['type'] ?? null) && array_key_exists('value', $v) && !$other) {
                    if (is_string($v['label'] ?? null) && $v['label'] !== '') $labels[$v['label']] = true;
                    foreach (is_array($v['anyOf'] ?? null) ? $v['anyOf'] : [] as $l) if (is_string($l) && $l !== '') $labels[$l] = true;
                }
            }
            foreach ($v as $child) if (is_array($child)) $stack[] = $child;
        }
        return ['variables' => $variables, 'labels' => $labels];
    }

    /**
     * The toon's dead things (@scutoons/content `deadThings`, the same rules) from its scan, variables judged across
     * every toon: set here and read in no toon = unread; read here and set in no toon = unset
     * @param array{sets: array<string, list<array<string, mixed>>>, reads: array<string, list<array<string, mixed>>>, points: list<array<string, mixed>>, labels: list<array<string, mixed>>} $scan
     * @param array<string, array{set?: array<string, true>, read?: array<string, true>}> $variables
     * @return array<string, list<array<string, mixed>>>
     */
    private function dead(string $toon, array $scan, array $variables): array
    {
        $list = function (array $map, callable $keep) use ($toon): array {
            $out = [];
            foreach ($map as $key => $at) if ($keep((string) $key)) $out[] = ['name' => Variables::written((string) $key, $toon), 'at' => $at];
            usort($out, fn($a, $b) => strcmp($a['name'], $b['name']));
            return $out;
        };
        return [
            'unread' => $list($scan['sets'], fn(string $n) => empty($variables[$n]['read'])),
            'unset' => $list($scan['reads'], fn(string $n) => empty($variables[$n]['set'])),
            'points' => $scan['points'],
            'labels' => $scan['labels'],
        ];
    }

    /**
     * One toon's chapters walked once: where variables are set / read, script points nothing diverts to, group labels
     * nothing refers to (`$facts['labels']`)
     * @param array<int, list<mixed>> $chapters
     * @param array{variables: array<string, true>, labels: array<string, true>} $facts
     * @return array{sets: array<string, list<array<string, mixed>>>, reads: array<string, list<array<string, mixed>>>, points: list<array<string, mixed>>, labels: list<array<string, mixed>>}
     */
    private function scan(string $toon, array $chapters, array $facts): array
    {
        $sets = [];
        $reads = [];
        $points = [];
        $labels = [];
        // keyed by storage keys (Variables): `x` here and `catpope.x` elsewhere are one variable
        $note = function (array &$map, string $name, array $at) use ($toon): void {
            if ($name === '' || str_starts_with($name, 'picked:')) return;
            $key = Variables::key($name, $toon);
            foreach ($map[$key] ?? [] as $p) if ($p['chapter'] === $at['chapter'] && $p['group'] === $at['group']) return;
            $map[$key][] = $at;
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
                        if (str_contains($v, '{')) foreach (self::storyNames(Analyser::analyseText($v)['reads']['names']) as $name) $note($reads, $name, $at);
                        continue;
                    }
                    if (!is_array($v)) continue;
                    if (!array_is_list($v)) {
                        $params = is_array($v['params'] ?? null) ? $v['params'] : [];
                        $type = $v['type'] ?? null;
                        if ($type === 'setVariable' && is_string($params['variable'] ?? null)) $note($sets, $params['variable'], $at);
                        if (($asked = self::askVariable($v)) !== null) $note($sets, $asked, $at);
                        foreach (['set', 'add'] as $op) {
                            if (is_array($v[$op] ?? null) && is_string($v[$op]['variable'] ?? null)) $note($sets, $v[$op]['variable'], $at);
                        }
                        if (is_array($v['if'] ?? null) && is_string($v['if']['variable'] ?? null)) $note($reads, $v['if']['variable'], $at);
                        if (is_array($v['if'] ?? null) && is_string($v['if']['expr'] ?? null)) {
                            foreach (self::storyNames(Analyser::analyseExpression($v['if']['expr'])['reads']['names']) as $name) $note($reads, $name, $at);
                        }
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
        return ['sets' => $sets, 'reads' => $reads, 'points' => $points, 'labels' => $labels];
    }

    /**
     * the story variable an askText event sets (@scutoons/content `askVariable`: scope toon – the default; scope reader
     * sets the reader's name, no variable)
     * @param array<mixed> $event
     */
    private static function askVariable(array $event): ?string
    {
        if (($event['type'] ?? null) !== 'askText') return null;
        $params = is_array($event['params'] ?? null) ? $event['params'] : [];
        if (($params['scope'] ?? null) === 'reader') return null;
        return is_string($params['variable'] ?? null) && $params['variable'] !== '' ? $params['variable'] : null;
    }

    /**
     * the story variables among the names an expression / a text reads, as written: `coins`, `global.coins`,
     * `toptoy.coins` (`reader.name` is none – Variables::isStoryPath)
     * @param list<list<string>> $names
     * @return list<string>
     */
    private static function storyNames(array $names): array
    {
        $out = [];
        foreach ($names as $path) if (Variables::isStoryPath($path)) $out[] = implode('.', $path);
        return $out;
    }

    /**
     * A chapter's pictures for the Flow (the same as @scutoons/content `chapterPictures`): its thumbnail, per group label
     * the low variant of the group's first slice (where a jump to that label lands), the first group's first slice.
     * @param list<mixed> $groups
     * @return array{thumbnail?: string, labels?: array<string, string>, first?: string}
     */
    private static function pictures(array $groups, mixed $thumbnail): array
    {
        $low = static function (mixed $group): ?string {
            $slice = is_array($group) && is_array($group['slices'] ?? null) ? ($group['slices'][0] ?? null) : null;
            $image = is_array($slice) && is_string($slice['image'] ?? null) ? trim($slice['image']) : '';
            if ($image === '') return null;
            $variant = is_array($slice['imageVariants'] ?? null) ? ($slice['imageVariants']['low'] ?? null) : null;
            return is_string($variant) && $variant !== '' ? $variant : $image;
        };
        $labels = [];
        foreach ($groups as $group) {
            $label = is_array($group) && is_string($group['label'] ?? null) ? $group['label'] : '';
            if ($label === '' || isset($labels[$label])) continue;
            $image = $low($group);
            if ($image !== null) $labels[$label] = $image;
        }
        $out = [];
        if (is_string($thumbnail) && $thumbnail !== '') $out['thumbnail'] = $thumbnail;
        if ($labels !== []) $out['labels'] = $labels;
        $first = $low($groups[0] ?? null);
        if ($first !== null) $out['first'] = $first;
        return $out;
    }

    /** @param array<mixed> $group a valid authored end (`groupEnd` in chains.ts) */
    private static function hasEnd(array $group): bool
    {
        return is_array($group['end'] ?? null) && self::target($group['end']['target'] ?? null) !== null;
    }
}
