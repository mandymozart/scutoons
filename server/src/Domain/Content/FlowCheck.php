<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Database\Database;
use stdClass;

/**
 * Where a reader could get stuck – refused on save (owner 2026-10-06: "Events queue takeovers have to be safe for
 * accidental dead ends" – avoided by logic and VALIDATION, no timers; the same rules as @scutoons/content stuck.ts and
 * validate.ts `checkTargetPlace` / `checkScriptPoints`, so MCP / the raw API can't store what the creator refuses):
 *   - a question whose choices can ALL be hidden (every addChoice before its showChoices has an `if`)
 *   - a divert (event, choice action, slice event) to a point that isn't in the group's script
 *   - a jump (jumpTo, a choice's jump, a group's end) to a chapter / group label / toon that doesn't exist – this
 *     toon's and other toons' (newest versions in the database; the document being saved counts with its new content)
 * Only neutral chapter documents (legacy per-locale ones are not written any more).
 */
final class FlowCheck
{
    /** @var array<string, list<string>|null> chapter `<toon>:<n>` → its group labels (null: no such chapter) */
    private array $labels = [];
    /** @var list<string> */
    private array $out = [];
    private EntityKey $key;
    private ?stdClass $doc = null;

    public function __construct(private readonly Database $db)
    {
    }

    /** Throws 422 invalid_content { problems } when the document can get the reader stuck */
    public function guard(EntityKey $key, mixed $content): void
    {
        $problems = $this->problems($key, $content);
        if ($problems) {
            throw new ApiError(422, 'invalid_content', "The $key->kind document can get the reader stuck: " . $problems[0]
                . (count($problems) > 1 ? ' (+' . (count($problems) - 1) . ' more)' : ''), ['problems' => $problems]);
        }
    }

    /** @return list<string> "<json pointer>: <message>" */
    public function problems(EntityKey $key, mixed $content, int $limit = 20): array
    {
        if (!$key->isNeutralDoc() || $key->kind !== 'chapter' || !$content instanceof stdClass) return [];
        $this->key = $key;
        $this->doc = $content;
        $this->out = [];
        $this->labels = [];
        foreach (is_array($content->groups ?? null) ? $content->groups : [] as $i => $group) {
            if ($group instanceof stdClass) $this->group($group, "/groups/$i");
        }
        return array_slice($this->out, 0, $limit);
    }

    private function group(stdClass $group, string $path): void
    {
        $points = [];
        foreach (is_array($group->events ?? null) ? $group->events : [] as $event) {
            if ($event instanceof stdClass && ($event->type ?? null) === 'label' && is_string($event->params->name ?? null) && $event->params->name !== '') {
                $points[$event->params->name] = true;
            }
        }
        $this->events($group->events ?? null, "$path/events", $points);
        foreach (is_array($group->slices ?? null) ? $group->slices : [] as $s => $slice) {
            if (!$slice instanceof stdClass) continue;
            $this->events($slice->events ?? null, "$path/slices/$s/events", $points);
            foreach (is_array($slice->items ?? null) ? $slice->items : [] as $k => $item) {
                if ($item instanceof stdClass) $this->events($item->events ?? null, "$path/slices/$s/items/$k/events", $points);
            }
        }
        if (($group->end ?? null) instanceof stdClass && ($group->end->target ?? null) instanceof stdClass) $this->target($group->end->target, "$path/end/target");
    }

    /** @param array<string, true> $points the group script's named points */
    private function events(mixed $events, string $path, array $points): void
    {
        $choices = [];
        foreach (is_array($events) ? $events : [] as $i => $event) {
            if (!$event instanceof stdClass) continue;
            $at = "$path/$i";
            $type = $event->type ?? null;
            $params = ($event->params ?? null) instanceof stdClass ? $event->params : new stdClass();
            $conditional = self::conditional($event);
            if ($type === 'divert') {
                $this->point($params->to ?? null, "$at/params/to", $points);
                if (!$conditional) $choices = [];
            } elseif ($type === 'jumpTo') {
                if (($params->target ?? null) instanceof stdClass) $this->target($params->target, "$at/params/target");
                if (!$conditional) $choices = [];
            } elseif ($type === 'addChoice') {
                $choices[] = $conditional;
                $actions = $params->onSelect ?? null;
                $actions = is_array($actions) ? $actions : ($actions instanceof stdClass ? [$actions] : []);
                foreach ($actions as $a => $action) {
                    if (!$action instanceof stdClass) continue;
                    $ap = "$at/params/onSelect" . (is_array($params->onSelect) ? "/$a" : '');
                    if (($action->divert ?? null) instanceof stdClass) $this->point($action->divert->to ?? null, "$ap/divert/to", $points);
                    if (($action->jumpTo ?? null) instanceof stdClass && ($action->jumpTo->target ?? null) instanceof stdClass) $this->target($action->jumpTo->target, "$ap/jumpTo/target");
                }
            } elseif ($type === 'showChoices') {
                if ($choices !== [] && !in_array(false, $choices, true)) {
                    $this->out[] = "$at: every choice of this question has a condition – all of them can be hidden at once, so the reader may get no choice: give one choice no condition (stuck:all-choices-hidden)";
                }
                $choices = [];
            }
        }
    }

    /** an event that runs only if its test holds (reach.ts `isConditional`) */
    private static function conditional(stdClass $event): bool
    {
        $if = $event->if ?? null;
        if (!$if instanceof stdClass) return false;
        return (is_string($if->variable ?? null) && $if->variable !== '') || (is_string($if->expr ?? null) && trim($if->expr) !== '');
    }

    /** @param array<string, true> $points */
    private function point(mixed $to, string $at, array $points): void
    {
        if (is_string($to) && $to !== '' && !isset($points[$to])) $this->out[] = "$at: there is no point \"$to\" in this group's script (stuck:missing-point)";
    }

    private function target(stdClass $target, string $at): void
    {
        $type = $target->type ?? null;
        if ($type === 'toon') {
            if (is_string($target->value ?? null) && $target->value !== '' && !$this->toonExists($target->value)) $this->out[] = "$at/value: there is no toon \"$target->value\" (stuck:missing-toon)";
            return;
        }
        if ($type !== 'chapter' || !is_scalar($target->value ?? null) || !preg_match('#^\d{1,6}$#', (string) $target->value)) return;
        $toon = is_string($target->toon ?? null) && $target->toon !== '' ? $target->toon : (string) $this->key->toon;
        $number = (int) $target->value;
        $other = $toon !== $this->key->toon;
        if ($other && !$this->toonExists($toon)) {
            $this->out[] = "$at/toon: there is no toon \"$toon\" (stuck:missing-toon)";
            return;
        }
        $labels = $this->labelsOf($toon, $number);
        $where = $other ? "$toon chapter $number" : "chapter $number";
        if ($labels === null) {
            $this->out[] = "$at/value: " . ($other ? "$toon has" : 'the toon has') . " no chapter $number (stuck:missing-chapter)";
            return;
        }
        $wanted = [];
        if (is_string($target->label ?? null) && $target->label !== '') $wanted[] = ['label', $target->label];
        foreach (is_array($target->anyOf ?? null) ? $target->anyOf : [] as $l) if (is_string($l) && $l !== '') $wanted[] = ['anyOf', $l];
        foreach ($wanted as [$field, $label]) {
            if (!in_array($label, $labels, true)) $this->out[] = "$at/$field: $where has no group labelled \"$label\" (stuck:missing-label)";
        }
    }

    private function toonExists(string $toon): bool
    {
        if ($toon === $this->key->toon) return true;
        return (bool) $this->db->value("SELECT 1 FROM entity_versions WHERE entity_key = ? OR entity_key LIKE ? LIMIT 1", ["toon:$toon", "chapter:$toon:%"]);
    }

    /**
     * a chapter's group labels: the newest version of each document; the document being saved
     * with its new content – null when the chapter doesn't exist
     * @return list<string>|null
     */
    private function labelsOf(string $toon, int $number): ?array
    {
        $id = "$toon:$number";
        if (array_key_exists($id, $this->labels)) return $this->labels[$id];
        $self = $this->key->toon === $toon && $this->key->number === $number;
        $chapter = $self && $this->key->kind === 'chapter' ? $this->doc : $this->newest("chapter:$toon:$number");
        if (!$chapter instanceof stdClass) return $this->labels[$id] = null;
        $labels = [];
        foreach (is_array($chapter->groups ?? null) ? $chapter->groups : [] as $group) {
            if (!$group instanceof stdClass) continue;
            if (is_string($group->label ?? null) && $group->label !== '') $labels[] = $group->label;
        }
        return $this->labels[$id] = $labels;
    }

    /** the newest version of an entity: its neutral document, else a legacy per-locale one */
    private function newest(string $entity): ?stdClass
    {
        $json = $this->db->value('SELECT json FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', [$entity])
            ?? $this->db->value('SELECT json FROM entity_versions WHERE entity_key LIKE ? ORDER BY id DESC LIMIT 1', ["$entity:%"]);
        $doc = is_string($json) ? json_decode($json) : null;
        return $doc instanceof stdClass ? $doc : null;
    }
}
