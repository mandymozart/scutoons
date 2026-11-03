<?php
declare(strict_types=1);

namespace Scutoons\Domain\Rigs;

use stdClass;

/**
 * Instruments, presets and rigs as deck-space reads them (#258, agents/research/INSTRUMENTS.md §1–3): the server refuses
 * what `deck_space::rig` would refuse, so a stored entry always plays on a deck. Mirrors, field by field:
 * - serde's forms (deny_unknown_fields: an unknown key is a problem; types; required fields; defaults may be left out),
 * - instrument.rs `Instrument::check` (a graph – graph.rs's kinds, ports, types, one link per input, no cycles – without
 *   scene-only kinds, `out` an audio output, every exposed path a control input, an `events` node's patch builds:
 *   deck-events patch.rs – known generators, edges, `out`, at most EVENT_KNOBS knobs, `listen`),
 * - preset.rs `PresetBook::lineage` (an unknown parent, a loop, a parent of another instrument) and `Instrument::patch`
 *   on the folded lineage (re-routed inputs exist, values on unlinked control inputs only, mapping targets, the graph),
 * - mod.rs `Rig::check` (unique track ids without ':' / '.', known instruments, presets of the track's instrument,
 *   variants and routes naming known tracks; `being:` only as a route's `to`).
 * The node kinds come from graph.rs (NodeKinds, generated – deck-space keeps it current); deck-space's
 * the_servers_scene_content_reads_as_bundles test parses the server's answer, so the two forms cannot drift.
 *
 * Data is decoded with objects (`json_decode($json)`: stdClass), so `{}` and `[]` stay apart – serde's maps refuse a list.
 * Lookups: `callable(string $id): ?stdClass` for instruments and presets (the catalog).
 */
final class RigValidator
{
    public const INSTRUMENT_ID = '#^[a-z0-9][a-z0-9_-]{0,63}$#';
    /** a factory preset: dot-separated slugs (`cathedral_voice.bright`), ≤ 100 characters */
    public const PRESET_ID = '#^(?=.{1,100}$)[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*){0,3}$#';
    /** a rig's id = its subject (KANON's form): `character.<id>[.<persona>]` or `item.<id>` */
    public const SUBJECT = '#^(?:character\.[a-z0-9][a-z0-9_-]{0,63}(?:\.[a-z0-9][a-z0-9_-]{0,63})?|item\.[A-Za-z0-9][A-Za-z0-9_-]{0,63})$#';
    /** instrument.rs InstrumentKind */
    public const KINDS = ['drum', 'bass', 'lead', 'pad', 'voice', 'sampler', 'texture', 'fx'];
    /** mod.rs Right */
    public const RIGHTS = ['play', 'tweak', 'derive', 'copy'];
    /** deck-events patch.rs KNOBS: an events node's knobs a, b, c, d */
    public const EVENT_KNOBS = 4;
    /** deck-events' generators: kind → [inputs, outputs, params] (generators/*.rs `ports`) */
    public const GENERATORS = [
        'random_walk' => [[], ['notes'], ['spread', 'rate', 'glide']],
        'pulse' => [[], ['out'], ['every']],
        'arp' => [['notes'], ['notes'], ['rate']],
    ];
    public const ARP_MODES = ['up', 'down', 'updown'];
    private const F32_MAX = 3.4028234663852886e38;

    // ── Instrument ────────────────────────────────────────────────────────────────────────────────────────

    /** @return list<string> */
    public static function instrument(mixed $i): array
    {
        if (!$i instanceof stdClass) return ['an instrument must be an object'];
        $id = is_string($i->id ?? null) ? $i->id : '?';
        $who = "instrument $id";
        $p = self::fields($who, $i, ['id', 'kind', 'name', 'node', 'link', 'out', 'expose', 'rights'], ['id', 'kind', 'node', 'out']);
        if (!is_string($i->id ?? null)) $p[] = "$who: id must be a string";
        elseif (!preg_match(self::INSTRUMENT_ID, $i->id)) $p[] = "$who: id must be lower-case letters, digits, _ or -";
        if (isset($i->kind) && !in_array($i->kind, self::KINDS, true)) $p[] = "$who: kind must be one of " . implode(', ', self::KINDS);
        if (isset($i->name)) $p = [...$p, ...self::text("$who: name", $i->name)];
        if (isset($i->rights)) $p = [...$p, ...self::rights($who, $i->rights)];
        if (isset($i->out) && !is_string($i->out)) $p[] = "$who: out must be a node.port";
        $nodes = self::nodes($who, $i->node ?? null, $p);
        $links = self::links("$who: link", $i->link ?? [], $p);
        if ($nodes === null || $links === null) return $p;
        $graph = self::graph($who, $nodes, $links);
        if ($graph === null) return [...$p, ...self::$graphProblems];
        foreach ($nodes as $n) {
            if (in_array($n['kind'], NodeKinds::SCENE_ONLY, true)) $p[] = "$who: a {$n['kind']} node belongs to the scene, not to an instrument ({$n['id']})";
        }
        if (is_string($i->out ?? null)) {
            [$node, $port] = self::split($i->out);
            $kind = $graph[$node] ?? null;
            if ($kind === null || (NodeKinds::KINDS[$kind]['outputs'][$port] ?? null) !== 'audio') $p[] = "$who: out '{$i->out}' is not an audio output";
        }
        $expose = $i->expose ?? [];
        if (!is_array($expose) || !array_is_list($expose)) $p[] = "$who: expose must be a list";
        else foreach ($expose as $n => $e) {
            if (!$e instanceof stdClass) {
                $p[] = "$who: expose[$n] must be { path, label?, range? }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: expose[$n]", $e, ['path', 'label', 'range'], ['path'])];
            if (!is_string($e->path ?? null)) continue;
            if (($problem = self::controlInput($who, $graph, $e->path)) !== null) $p[] = $problem;
            if (isset($e->label)) $p = [...$p, ...self::text("$who: {$e->path} label", $e->label)];
            if (isset($e->range) && !self::range($e->range)) $p[] = "$who: {$e->path} range must be [min, max] (numbers)";
        }
        foreach ($nodes as $n) {
            if ($n['kind'] === 'events') $p = [...$p, ...self::events("$who: events node '{$n['id']}'", $n['params'])];
        }
        return $p;
    }

    // ── Preset ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * A preset against its instrument and lineage. $presets sees the catalog – this entry
     * counts with its new data.
     * @param callable(string): ?stdClass $instruments
     * @param callable(string): ?stdClass $presets
     * @return list<string>
     */
    public static function preset(mixed $pr, callable $instruments, callable $presets, string $idRule = self::PRESET_ID): array
    {
        if (!$pr instanceof stdClass) return ['a preset must be an object'];
        $id = is_string($pr->id ?? null) ? $pr->id : '?';
        $who = "preset $id";
        $p = self::fields($who, $pr, ['id', 'instrument', 'parent', 'name', 'tags', 'values', 'unlink', 'link', 'mapping', 'rights'], ['id', 'instrument']);
        if (!is_string($pr->id ?? null) || !preg_match($idRule, $pr->id)) $p[] = "$who: id is not a valid preset id";
        if (isset($pr->name)) $p = [...$p, ...self::text("$who: name", $pr->name)];
        if (isset($pr->rights)) $p = [...$p, ...self::rights($who, $pr->rights)];
        if (isset($pr->tags) && !self::strings($pr->tags)) $p[] = "$who: tags must be a list of strings";
        if (isset($pr->parent) && !is_string($pr->parent)) $p[] = "$who: parent must be a preset id";
        if (isset($pr->values) && !($pr->values instanceof stdClass && array_filter(get_object_vars($pr->values), fn($v) => !self::number($v)) === [])) $p[] = "$who: values must be { \"node.port\": number }";
        if (isset($pr->unlink) && !self::strings($pr->unlink)) $p[] = "$who: unlink must be a list of node.port";
        $links = self::links("$who: link", $pr->link ?? [], $p);
        $mappings = $pr->mapping ?? [];
        if (!is_array($mappings) || !array_is_list($mappings)) $p[] = "$who: mapping must be a list";
        else foreach ($mappings as $n => $m) {
            if (!$m instanceof stdClass) {
                $p[] = "$who: mapping[$n] must be { source, target, range? }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: mapping[$n]", $m, ['source', 'target', 'range'], ['source', 'target'])];
            if (isset($m->source) && (!is_string($m->source) || $m->source === '')) $p[] = "$who: mapping[$n].source must be a source (midi:<ch>:cc:<n>, face:…, motion:…, pad:…, stick:…)";
            if (isset($m->target) && !is_string($m->target)) $p[] = "$who: mapping[$n].target must be a node.port";
            if (isset($m->range) && !self::range($m->range)) $p[] = "$who: mapping[$n].range must be [min, max] (numbers)";
        }
        if (!is_string($pr->instrument ?? null)) return [...$p, "$who: instrument must be an instrument id"];
        $instrument = $instruments($pr->instrument);
        if ($instrument === null) return [...$p, "$who: unknown instrument \"{$pr->instrument}\""];
        if ($p !== [] || $links === null) return $p;
        $lookup = fn(string $pid) => $pid === $id ? $pr : $presets($pid);
        $chain = self::lineage($id, $lookup, $p);
        if ($chain === null) return $p;
        return self::patch($who, $instrument, $chain);
    }

    /**
     * `id` and its ancestors, root first (preset.rs `lineage`); null with the problem added when it does not resolve
     * @param callable(string): ?stdClass $presets
     * @param list<string> $p
     * @return list<stdClass>|null
     */
    public static function lineage(string $id, callable $presets, array &$p): ?array
    {
        $first = $presets($id);
        if ($first === null) {
            $p[] = "unknown preset \"$id\"";
            return null;
        }
        $chain = [$first];
        $ids = [$id];
        while (is_string($chain[count($chain) - 1]->parent ?? null)) {
            $child = $chain[count($chain) - 1];
            $parent = $child->parent;
            if (in_array($parent, $ids, true)) {
                $p[] = "preset $id: its lineage loops at \"$parent\"";
                return null;
            }
            $next = $presets($parent);
            if ($next === null) {
                $p[] = "preset {$child->id}: unknown preset \"$parent\" (its parent)";
                return null;
            }
            if (($next->instrument ?? null) !== ($child->instrument ?? null)) {
                $p[] = "preset {$child->id} (" . self::show($child->instrument ?? null) . ") derives from \"$parent\" of another instrument (" . self::show($next->instrument ?? null) . ')';
                return null;
            }
            $chain[] = $next;
            $ids[] = $parent;
        }
        return array_reverse($chain);
    }

    /**
     * instrument.rs `patch` on a folded chain: re-routed inputs exist, values only on unlinked control inputs, mapping
     * targets are control inputs, the patched graph builds
     * @param list<stdClass> $chain root first
     * @return list<string>
     */
    private static function patch(string $who, stdClass $instrument, array $chain): array
    {
        $p = [];
        $nodes = self::nodes("instrument " . self::show($instrument->id ?? null), $instrument->node ?? null, $p);
        $base = self::links('instrument link', $instrument->link ?? [], $p);
        if ($nodes === null || $base === null || $p !== []) return ["$who: its instrument is not valid"];
        $kinds = array_column($nodes, 'kind', 'id');
        // fold root → leaf (preset.rs Resolved::fold)
        $values = [];
        $unlink = [];
        $added = [];
        $mappings = [];
        foreach ($chain as $pr) {
            foreach (get_object_vars($pr->values ?? new stdClass()) as $k => $v) $values[self::canonical((string) $k)] = $v;
            foreach ((array) ($pr->unlink ?? []) as $u) {
                $u = self::canonical((string) $u);
                unset($added[$u]);
                $unlink[$u] = true;
            }
            foreach ((array) ($pr->link ?? []) as $l) {
                $to = self::canonical((string) ($l->to ?? ''));
                unset($unlink[$to]);
                $added[$to] = ['from' => (string) ($l->from ?? ''), 'to' => (string) ($l->to ?? '')];
            }
            foreach ((array) ($pr->mapping ?? []) as $m) $mappings[($m->source ?? '') . "\0" . self::canonical((string) ($m->target ?? ''))] = (string) ($m->target ?? '');
        }
        foreach ([...array_keys($unlink), ...array_keys($added)] as $target) {
            [$node, $port] = self::split((string) $target);
            $kind = $kinds[$node] ?? null;
            if ($kind === null || !isset(NodeKinds::KINDS[$kind]['inputs'][$port])) $p[] = "$who: no input $target to re-route";
        }
        if ($p !== []) return $p;
        $links = array_values(array_filter($base, fn(array $l) => !isset($unlink[self::canonical($l['to'])]) && !isset($added[self::canonical($l['to'])])));
        $links = [...$links, ...array_values($added)];
        foreach ($values as $path => $v) {
            if (($problem = self::controlInput($who, $kinds, (string) $path)) !== null) $p[] = $problem;
            elseif (array_filter($links, fn(array $l) => self::canonical($l['to']) === $path) !== []) $p[] = "$who: $path is linked – a preset value needs an unlinked input (unlink it first)";
        }
        foreach ($mappings as $target) {
            if (($problem = self::controlInput($who, $kinds, $target)) !== null) $p[] = $problem;
        }
        if ($p !== []) return $p;
        if (self::graph("$who (with its instrument)", $nodes, $links) === null) return self::$graphProblems;
        return [];
    }

    // ── Rig ───────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * @param callable(string): ?stdClass $instruments
     * @param callable(string): ?stdClass $presets
     * @param list<string>|null $allowed the instruments it may use (a member's rig: the ones they hold); null = any
     * @return list<string>
     */
    public static function rig(mixed $r, string $id, callable $instruments, callable $presets, ?array $allowed = null): array
    {
        if (!$r instanceof stdClass) return ['a rig must be an object'];
        $who = "rig $id";
        $p = self::fields($who, $r, ['subject', 'track', 'route', 'personas', 'moods', 'rights', 'sticks'], ['subject', 'track']);
        if (!preg_match(self::SUBJECT, $id)) $p[] = "$who: the id is a subject – character.<id>[.<persona>] or item.<id>";
        if (($r->subject ?? null) !== $id) $p[] = "$who: subject must be \"$id\"";
        if (isset($r->rights)) $p = [...$p, ...self::rights($who, $r->rights)];
        $tracks = [];
        $list = $r->track ?? null;
        if (!is_array($list) || !array_is_list($list)) return [...$p, "$who: track must be a list"];
        foreach ($list as $n => $t) {
            if (!$t instanceof stdClass) {
                $p[] = "$who: track[$n] must be { id, instrument, preset?, gain?, pan?, mute? }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: track[$n]", $t, ['id', 'instrument', 'preset', 'gain', 'pan', 'mute'], ['id', 'instrument'])];
            if (!is_string($t->id ?? null) || !is_string($t->instrument ?? null)) continue;
            if (isset($tracks[$t->id])) $p[] = "$who: track '{$t->id}' twice";
            if (str_contains($t->id, ':') || str_contains($t->id, '.')) $p[] = "$who: a track id has no ':' or '.' ({$t->id})";
            foreach (['gain', 'pan'] as $f) if (isset($t->$f) && !self::number($t->$f)) $p[] = "$who: track {$t->id}: $f must be a number";
            if (isset($t->mute) && !is_bool($t->mute)) $p[] = "$who: track {$t->id}: mute must be true or false";
            if (isset($t->preset) && !is_string($t->preset)) $p[] = "$who: track {$t->id}: preset must be a preset id";
            $instrument = $instruments($t->instrument);
            if ($instrument === null) $p[] = "$who: unknown instrument \"{$t->instrument}\"";
            elseif ($allowed !== null && !in_array($t->instrument, $allowed, true)) $p[] = "$who: track {$t->id}: the instrument \"{$t->instrument}\" is not one held";
            $tracks[$t->id] = ['instrument' => $t->instrument, 'def' => $instrument, 'mute' => ($t->mute ?? false) === true];
            if (is_string($t->preset ?? null)) $p = [...$p, ...self::fits($who, $t->id, $t->instrument, $t->preset, $presets)];
        }
        foreach (['personas' => 'persona', 'moods' => 'mood'] as $field => $label) {
            if (!isset($r->$field)) continue;
            if (!$r->$field instanceof stdClass) {
                $p[] = "$who: $field must be { <$label>: { presets?, values? } }";
                continue;
            }
            foreach (get_object_vars($r->$field) as $name => $v) {
                $at = "$who: $label $name";
                if (!$v instanceof stdClass) {
                    $p[] = "$at must be { presets?, values? }";
                    continue;
                }
                $p = [...$p, ...self::fields($at, $v, ['presets', 'values'], [])];
                if (isset($v->presets)) {
                    if (!$v->presets instanceof stdClass) $p[] = "$at: presets must be { <track>: <preset> }";
                    else foreach (get_object_vars($v->presets) as $track => $preset) {
                        if (!isset($tracks[$track])) $p[] = "$at: no track '$track'";
                        elseif (!is_string($preset)) $p[] = "$at: presets.$track must be a preset id";
                        else $p = [...$p, ...self::fits($at, (string) $track, $tracks[$track]['instrument'], $preset, $presets)];
                    }
                }
                if (isset($v->values)) {
                    if (!$v->values instanceof stdClass) $p[] = "$at: values must be { \"<track>:<node>.<port>\": number }";
                    else foreach (get_object_vars($v->values) as $path => $value) {
                        $path = (string) $path;
                        if (!self::number($value)) $p[] = "$at: $path must be a number";
                        $parts = explode(':', $path, 2);
                        if (count($parts) !== 2) $p[] = "$at: '$path' is not <track>:<node>.<port>";
                        elseif (!isset($tracks[$parts[0]])) $p[] = "$at: no track '{$parts[0]}'";
                        elseif ($tracks[$parts[0]]['def'] !== null) {
                            $kinds = self::kindsOf($tracks[$parts[0]]['def']);
                            if (($problem = self::controlInput("instrument {$tracks[$parts[0]]['instrument']}", $kinds, $parts[1])) !== null) $p[] = "$at: $problem";
                        }
                    }
                }
            }
        }
        if (isset($r->sticks)) $p = [...$p, ...self::sticks($who, $r->sticks, $tracks)];
        $routes = $r->route ?? [];
        if (!is_array($routes) || !array_is_list($routes)) $p[] = "$who: route must be a list";
        else foreach ($routes as $n => $route) {
            if (!$route instanceof stdClass) {
                $p[] = "$who: route[$n] must be { from, to, scale?, offset? }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: route[$n]", $route, ['from', 'to', 'scale', 'offset'], ['from', 'to'])];
            foreach (['scale', 'offset'] as $f) if (isset($route->$f) && !self::number($route->$f)) $p[] = "$who: route[$n].$f must be a number";
            foreach (['from', 'to'] as $end) {
                $ref = $route->$end ?? null;
                if (!is_string($ref)) continue;
                $parts = explode(':', $ref, 2);
                if (count($parts) !== 2) $p[] = "$who: route end '$ref' has no scope";
                elseif (!($parts[0] === 'being' && $end === 'to') && !isset($tracks[$parts[0]])) $p[] = "$who: no track '{$parts[0]}' (route $ref)";
            }
        }
        return $p;
    }

    /**
     * mod.rs `sticks` + bundle.rs `play`: `[[x, y], [x, y]]` knob paths, each "" or a knob – an exposed parameter of an
     * unmuted track, `<track>_<node>.<port>`
     * @param array<string, array{instrument: string, def: ?stdClass, mute: bool}> $tracks
     * @return list<string>
     */
    private static function sticks(string $who, mixed $sticks, array $tracks): array
    {
        $pair = fn(mixed $x) => is_array($x) && array_is_list($x) && count($x) === 2 && is_string($x[0]) && is_string($x[1]);
        if (!(is_array($sticks) && array_is_list($sticks) && count($sticks) === 2 && $pair($sticks[0]) && $pair($sticks[1]))) {
            return ["$who: sticks must be [[x, y], [x, y]] (knob paths, \"\" = none)"];
        }
        $knobs = [];
        foreach ($tracks as $id => $t) {
            if ($t['mute'] || $t['def'] === null) continue;
            foreach ((array) ($t['def']->expose ?? []) as $e) {
                if ($e instanceof stdClass && is_string($e->path ?? null)) $knobs[] = $id . '_' . self::canonical($e->path);
            }
        }
        $p = [];
        foreach ([...$sticks[0], ...$sticks[1]] as $path) {
            if ($path !== '' && !in_array($path, $knobs, true)) $p[] = "$who: stick '$path' is not one of its knobs";
        }
        return $p;
    }

    /**
     * mod.rs `preset_fits`: a known preset of the track's instrument
     * @param callable(string): ?stdClass $presets
     * @return list<string>
     */
    private static function fits(string $who, string $track, string $instrument, string $preset, callable $presets): array
    {
        $pr = $presets($preset);
        if ($pr === null) return ["$who: unknown preset \"$preset\""];
        if (($pr->instrument ?? null) !== $instrument) return ["$who: track '$track' plays '$instrument', preset '$preset' is for '" . self::show($pr->instrument ?? null) . "'"];
        return [];
    }

    // ── Graph (graph.rs `Graph::build`) ───────────────────────────────────────────────────────────────────

    /** @var list<string> the problems of the last graph() that failed */
    private static array $graphProblems = [];

    /**
     * Node id → kind when the nodes and links form a valid graph (graph.rs: known kinds, unique ids, at most one of each
     * sink, links from an output to an input of the same type, one link per input, no cycle over the ordering links);
     * null with the problems in $graphProblems otherwise.
     * @param list<array{id: string, kind: string, params: array<string, mixed>}> $nodes
     * @param list<array{from: string, to: string}> $links
     * @return array<string, string>|null
     */
    private static function graph(string $who, array $nodes, array $links): ?array
    {
        $p = [];
        $kinds = [];
        $index = [];
        foreach ($nodes as $n) {
            if (!isset(NodeKinds::KINDS[$n['kind']])) $p[] = "$who: node '{$n['id']}': unknown kind '{$n['kind']}'";
            if (isset($kinds[$n['id']])) $p[] = "$who: node id '{$n['id']}' is used twice";
            $index[$n['id']] = count($index);
            $kinds[$n['id']] = $n['kind'];
        }
        foreach (NodeKinds::SCENE_ONLY as $sink) {
            if (count(array_filter($kinds, fn(string $k) => $k === $sink)) > 1) $p[] = "$who: at most one $sink node";
        }
        if ($p !== []) {
            self::$graphProblems = $p;
            return null;
        }
        $edges = [];
        $inputs = [];
        foreach ($links as $l) {
            [$fn, $fp] = self::split($l['from']);
            [$tn, $tp] = self::split($l['to']);
            $at = "$who: link {$l['from']} → {$l['to']}";
            if (!isset($kinds[$fn])) { $p[] = "$at: no node '$fn'"; continue; }
            if (!isset($kinds[$tn])) { $p[] = "$at: no node '$tn'"; continue; }
            $fty = NodeKinds::KINDS[$kinds[$fn]]['outputs'][$fp] ?? null;
            $tty = NodeKinds::KINDS[$kinds[$tn]]['inputs'][$tp] ?? null;
            if ($fty === null) { $p[] = "$at: {$kinds[$fn]} has no output '$fp'"; continue; }
            if ($tty === null) { $p[] = "$at: {$kinds[$tn]} has no input '$tp'"; continue; }
            if ($fty !== $tty) { $p[] = "$at: a $fty output cannot feed a $tty input"; continue; }
            if (isset($inputs["$tn.$tp"])) { $p[] = "$at: the input already has a link"; continue; }
            $inputs["$tn.$tp"] = true;
            // Link::orders: a control link into or out of an audio node crosses threads a frame late – it never orders
            $orders = $fty !== 'control' || (NodeKinds::KINDS[$kinds[$fn]]['domain'] !== 'audio' && NodeKinds::KINDS[$kinds[$tn]]['domain'] !== 'audio');
            if ($orders) $edges[] = [$index[$fn], $index[$tn]];
        }
        if ($p === [] && self::cyclic(count($nodes), $edges)) $p[] = "$who: the graph has a cycle";
        if ($p !== []) {
            self::$graphProblems = $p;
            return null;
        }
        return $kinds;
    }

    /** @param list<array{0: int, 1: int}> $edges */
    private static function cyclic(int $n, array $edges): bool
    {
        $indeg = array_fill(0, $n, 0);
        foreach ($edges as [, $to]) $indeg[$to]++;
        $ready = array_keys(array_filter($indeg, fn(int $d) => $d === 0));
        $done = 0;
        while ($ready !== []) {
            $next = array_shift($ready);
            $done++;
            foreach ($edges as [$from, $to]) {
                if ($from === $next && --$indeg[$to] === 0) $ready[] = $to;
            }
        }
        return $done < $n;
    }

    /**
     * deck-events patch.rs `Patch::from_table` + `build` on an events node's params
     * @param array<string, mixed> $params
     * @return list<string>
     */
    private static function events(string $who, array $params): array
    {
        $p = [];
        $generators = $params['generator'] ?? [];
        if (!is_array($generators) || !array_is_list($generators)) return ["$who: generator must be a list"];
        $ports = [];
        foreach ($generators as $n => $g) {
            if (!$g instanceof stdClass || !is_string($g->id ?? null) || !is_string($g->kind ?? null)) {
                $p[] = "$who: generator[$n] must be { id, kind, … }";
                continue;
            }
            if (isset($ports[$g->id])) $p[] = "$who: generator '{$g->id}' twice";
            if (!isset(self::GENERATORS[$g->kind])) {
                $p[] = "$who: generator '{$g->id}': unknown kind '{$g->kind}' (" . implode(', ', array_keys(self::GENERATORS)) . ')';
                continue;
            }
            if ($g->kind === 'arp' && isset($g->mode) && !in_array($g->mode, self::ARP_MODES, true)) $p[] = "$who: {$g->id}: unknown mode '" . self::show($g->mode) . "' (up, down, updown)";
            if ($g->kind === 'pulse' && isset($g->name) && !(is_string($g->name) && self::eventName($g->name))) $p[] = "$who: {$g->id}: name must be an event name (domain.name)";
            $ports[$g->id] = self::GENERATORS[$g->kind];
        }
        // "id.port" → [ports of the generator, port]
        $ref = function (mixed $r, int $slot, string $what) use ($who, $ports, &$p): bool {
            if (!is_string($r) || !str_contains($r, '.')) {
                $p[] = "$who: '" . self::show($r) . "' is not <generator>.<port>";
                return false;
            }
            $cut = (int) strrpos($r, '.');
            [$g, $port] = [substr($r, 0, $cut), substr($r, $cut + 1)];
            if (!isset($ports[$g])) {
                $p[] = "$who: no generator '$g' ($r)";
                return false;
            }
            if (!in_array($port, $ports[$g][$slot], true)) {
                $p[] = "$who: $g has no $what '$port'";
                return false;
            }
            return true;
        };
        $edges = $params['edge'] ?? [];
        if (!is_array($edges) || !array_is_list($edges)) $p[] = "$who: edge must be a list";
        else foreach ($edges as $n => $e) {
            if (!$e instanceof stdClass) {
                $p[] = "$who: edge[$n] must be { from, to, delayed? }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: edge[$n]", $e, ['from', 'to', 'delayed'], ['from', 'to'])];
            $ref($e->from ?? null, 1, 'output');
            $ref($e->to ?? null, 0, 'input');
            if (isset($e->delayed) && !is_bool($e->delayed)) $p[] = "$who: edge[$n].delayed must be true or false";
        }
        if (!array_key_exists('out', $params)) $p[] = "$who: out is required (<generator>.<port> the host plays)";
        else $ref($params['out'], 1, 'output');
        $knobs = $params['knobs'] ?? [];
        if (!is_array($knobs) || !array_is_list($knobs)) $p[] = "$who: knobs must be a list";
        elseif (count($knobs) > self::EVENT_KNOBS) $p[] = "$who: at most " . self::EVENT_KNOBS . ' knobs (a, b, c, d), not ' . count($knobs);
        else foreach ($knobs as $k) $ref($k, 2, 'param');
        $listen = $params['listen'] ?? [];
        if (!is_array($listen) || !array_is_list($listen)) $p[] = "$who: listen must be a list";
        else foreach ($listen as $n => $l) {
            if (!$l instanceof stdClass) {
                $p[] = "$who: listen[$n] must be { to, name | domain }";
                continue;
            }
            $p = [...$p, ...self::fields("$who: listen[$n]", $l, ['to', 'name', 'domain'], ['to'])];
            $ref($l->to ?? null, 0, 'input');
            $name = $l->name ?? null;
            $domain = $l->domain ?? null;
            if (($name === null) === ($domain === null)) $p[] = "$who: listen[$n]: a `name` or a `domain`, one of them";
            elseif ($name !== null && !(is_string($name) && self::eventName($name))) $p[] = "$who: listen[$n].name must be an event name (domain.name)";
            elseif ($domain !== null && !(is_string($domain) && preg_match('/^[a-z0-9_]+$/', $domain))) $p[] = "$who: listen[$n].domain must be one plain word";
        }
        if ($p === [] && self::eventCycle($edges, $ports)) $p[] = "$who: the generators loop without a delayed edge";
        return $p;
    }

    /**
     * @param list<mixed> $edges
     * @param array<string, mixed> $ports
     */
    private static function eventCycle(array $edges, array $ports): bool
    {
        $index = array_flip(array_keys($ports));
        $pairs = [];
        foreach ($edges as $e) {
            if (($e->delayed ?? false) === true) continue;
            $from = substr((string) $e->from, 0, (int) strrpos((string) $e->from, '.'));
            $to = substr((string) $e->to, 0, (int) strrpos((string) $e->to, '.'));
            $pairs[] = [$index[$from], $index[$to]];
        }
        return self::cyclic(count($ports), $pairs);
    }

    /** names.rs `event`: a domain word, then parts (camelCase allowed) */
    private static function eventName(string $name): bool
    {
        return preg_match('/^[a-z0-9_]+(?:\.[a-z][A-Za-z0-9_]*)+$/', $name) === 1;
    }

    // ── Pieces ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * The nodes of an instrument as { id, kind, params } (scene.rs NodeDef: id + kind + the kind's params, which
     * become TOML – no null anywhere); null when they are not a list of such
     * @param list<string> $p
     * @return list<array{id: string, kind: string, params: array<string, mixed>}>|null
     */
    private static function nodes(string $who, mixed $list, array &$p): ?array
    {
        if (!is_array($list) || !array_is_list($list)) {
            $p[] = "$who: node must be a list";
            return null;
        }
        $out = [];
        foreach ($list as $n => $node) {
            if (!$node instanceof stdClass || !is_string($node->id ?? null) || !is_string($node->kind ?? null)) {
                $p[] = "$who: node[$n] must be { id, kind, … }";
                continue;
            }
            $params = get_object_vars($node);
            unset($params['id'], $params['kind']);
            foreach ($params as $k => $v) {
                if (self::hasNull($v)) $p[] = "$who: node {$node->id}: $k has a null (TOML has none – leave it out)";
            }
            $out[] = ['id' => $node->id, 'kind' => $node->kind, 'params' => $params];
        }
        return count($out) === count($list) ? $out : null;
    }

    /**
     * scene.rs LinkDef { from, to, scale?, offset? }
     * @param list<string> $p
     * @return list<array{from: string, to: string}>|null
     */
    private static function links(string $who, mixed $list, array &$p): ?array
    {
        if (!is_array($list) || !array_is_list($list)) {
            $p[] = "$who must be a list";
            return null;
        }
        $out = [];
        foreach ($list as $n => $l) {
            if (!$l instanceof stdClass) {
                $p[] = "{$who}[$n] must be { from, to, scale?, offset? }";
                continue;
            }
            $before = count($p);
            $p = [...$p, ...self::fields("{$who}[$n]", $l, ['from', 'to', 'scale', 'offset'], ['from', 'to'])];
            if (isset($l->from) && !is_string($l->from)) $p[] = "{$who}[$n].from must be a node.port";
            if (isset($l->to) && !is_string($l->to)) $p[] = "{$who}[$n].to must be a node.port";
            foreach (['scale', 'offset'] as $f) if (isset($l->$f) && !self::number($l->$f)) $p[] = "{$who}[$n].$f must be a number";
            if (count($p) === $before) $out[] = ['from' => (string) $l->from, 'to' => (string) $l->to];
        }
        return count($out) === count($list) ? $out : null;
    }

    /**
     * An instrument's node id → kind (unchecked)
     * @return array<string, string>
     */
    private static function kindsOf(stdClass $instrument): array
    {
        $out = [];
        foreach ((array) ($instrument->node ?? []) as $n) {
            if ($n instanceof stdClass && is_string($n->id ?? null) && is_string($n->kind ?? null)) $out[$n->id] = $n->kind;
        }
        return $out;
    }

    /**
     * instrument.rs `control_input`: `node.port` names a control input of the node's kind
     * @param array<string, string> $kinds node id → kind
     */
    private static function controlInput(string $who, array $kinds, string $path): ?string
    {
        [$node, $port] = self::split($path);
        $kind = $kinds[$node] ?? null;
        if ($kind === null) return "$who: no node '$node' ($path)";
        if (!isset(NodeKinds::KINDS[$kind])) return "$who: unknown kind '$kind'";
        if ((NodeKinds::KINDS[$kind]['inputs'][$port] ?? null) !== 'control') return "$who: $path is not a control input of $kind";
        return null;
    }

    /**
     * serde's deny_unknown_fields + the required fields
     * @param list<string> $known
     * @param list<string> $required
     * @return list<string>
     */
    private static function fields(string $who, stdClass $o, array $known, array $required): array
    {
        $p = [];
        foreach (array_keys(get_object_vars($o)) as $k) {
            if (!in_array($k, $known, true)) $p[] = "$who: unknown field \"$k\" (" . implode(', ', $known) . ')';
        }
        foreach ($required as $k) {
            if (!isset($o->$k)) $p[] = "$who: $k is required";
        }
        return $p;
    }

    /** @return list<string> mod.rs Rights { lent, owned, core } – lists of play · tweak · derive · copy */
    private static function rights(string $who, mixed $r): array
    {
        if (!$r instanceof stdClass) return ["$who: rights must be { lent?, owned?, core? }"];
        $p = self::fields("$who: rights", $r, ['lent', 'owned', 'core'], []);
        foreach (get_object_vars($r) as $hold => $list) {
            if (!is_array($list) || !array_is_list($list) || array_diff($list, self::RIGHTS) !== []) $p[] = "$who: rights.$hold must be a list of " . implode(', ', self::RIGHTS);
        }
        return $p;
    }

    /** @return list<string> a locale map `{ de, en }` (mod.rs Text) */
    private static function text(string $who, mixed $t): array
    {
        if (!$t instanceof stdClass || array_filter(get_object_vars($t), fn($v) => !is_string($v)) !== []) return ["$who must be { <locale>: text }"];
        return [];
    }

    private static function range(mixed $r): bool
    {
        return is_array($r) && array_is_list($r) && count($r) === 2 && self::number($r[0]) && self::number($r[1]);
    }

    private static function strings(mixed $l): bool
    {
        return is_array($l) && array_is_list($l) && array_filter($l, fn($s) => !is_string($s)) === [];
    }

    /** an f32 (serde reads JSON numbers into f32 – past its range they would turn infinite) */
    private static function number(mixed $v): bool
    {
        return (is_int($v) || is_float($v)) && is_finite((float) $v) && abs((float) $v) <= self::F32_MAX;
    }

    private static function hasNull(mixed $v): bool
    {
        if ($v === null) return true;
        if ($v instanceof stdClass) $v = get_object_vars($v);
        if (is_array($v)) foreach ($v as $x) if (self::hasNull($x)) return true;
        return false;
    }

    /** @return array{0: string, 1: string} scene.rs `split_ref`: `node.port`, a bare node = its `out` */
    public static function split(string $ref): array
    {
        $at = strpos($ref, '.');
        return $at === false ? [$ref, 'out'] : [substr($ref, 0, $at), substr($ref, $at + 1)];
    }

    /** instrument.rs `canonical`: `node` → `node.out` */
    public static function canonical(string $ref): string
    {
        [$n, $p] = self::split($ref);
        return "$n.$p";
    }

    private static function show(mixed $v): string
    {
        return is_scalar($v) ? (string) $v : (string) json_encode($v);
    }
}
