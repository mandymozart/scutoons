<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content\Expr;

use stdClass;

/**
 * Expressions in the texts of a language-neutral chapter document (agents/EXPRESSIONS.md §4, step 2): every text
 * box line (`slices[].dialog[].text`), VN message (`showMessage` `params.text`) and choice label (`addChoice`
 * `params.label`), askText question (`params.text`) per language and an event condition's expression (`if.expr`), in the
 * group's script, the slices' events and the placed items' events. Only what
 * can't work blocks a save: syntax errors and the old clock form `{x:clock}` (no alias – migration 2026-10-06-clock-syntax).
 * What the analyser can only doubt (unknown names / functions, arguments) the creator shows as warnings
 * (@scutoons/content validate.ts `checkTextExpressions`, the same codes).
 */
final class TextCheck
{
    /** problems that are only warnings (validate.ts EXPR_DOUBTS) */
    private const DOUBTS = ['unknown-name', 'unknown-variable', 'unknown-function', 'function-needs-call', 'not-a-function', 'namespace-needs-member', 'argument-count', 'argument-type'];

    /** @var list<string> */
    private array $out = [];

    /**
     * The blocking problems of a neutral document: "<json pointer>: <message> – at <from>–<to> (expr:<code>)".
     *
     * @param string $kind `chapter` (its `groups`); other kinds have no texts checked here
     * @return list<string>
     */
    public static function problems(string $kind, mixed $doc, int $limit = 20): array
    {
        $check = new self();
        if (!$doc instanceof stdClass) return [];
        if ($kind === 'chapter') {
            foreach (is_array($doc->groups ?? null) ? $doc->groups : [] as $i => $group) $check->group($group, "/groups/$i");
        }
        return array_slice($check->out, 0, $limit);
    }

    private function group(mixed $group, string $path): void
    {
        if (!$group instanceof stdClass) return;
        foreach (is_array($group->slices ?? null) ? $group->slices : [] as $s => $slice) {
            if (!$slice instanceof stdClass) continue;
            foreach (is_array($slice->dialog ?? null) ? $slice->dialog : [] as $d => $dialog) {
                if ($dialog instanceof stdClass) $this->localized($dialog->text ?? null, "$path/slices/$s/dialog/$d/text");
            }
            $this->events($slice->events ?? null, "$path/slices/$s/events");
            foreach (is_array($slice->items ?? null) ? $slice->items : [] as $it => $item) {
                if ($item instanceof stdClass) $this->events($item->events ?? null, "$path/slices/$s/items/$it/events");
            }
        }
        $this->events($group->events ?? null, "$path/events");
    }
    private function events(mixed $events, string $path): void
    {
        foreach (is_array($events) ? $events : [] as $e => $event) {
            if (!$event instanceof stdClass) continue;
            // an `if` as an expression (the advanced form): a syntax error can't be stored
            if (($event->if ?? null) instanceof stdClass && is_string($event->if->expr ?? null)) $this->expression($event->if->expr, "$path/$e/if/expr");
            if (!($event->params ?? null) instanceof stdClass) continue;
            $field = match ($event->type ?? null) {
                'showMessage', 'askText' => 'text',
                'addChoice' => 'label',
                default => null,
            };
            if ($field !== null) $this->localized($event->params->{$field} ?? null, "$path/$e/params/$field");
        }
    }

    /** a locale map { de: text | lines } (or an older plain text) */
    private function localized(mixed $value, string $path): void
    {
        if ($value instanceof stdClass) {
            foreach (get_object_vars($value) as $locale => $text) $this->text($text, "$path/$locale");
        } else {
            $this->text($value, $path);
        }
    }
    /** an event condition's expression (`if: { expr }`): its blocking problems */
    private function expression(string $source, string $path): void
    {
        foreach (Analyser::analyseExpression($source)['errors'] as $p) {
            if ($p['severity'] !== 'error' || in_array($p['code'], self::DOUBTS, true)) continue;
            $this->out[] = "$path: {$p['message']} – at " . ($p['start'] + 1) . ($p['end'] > $p['start'] + 1 ? '–' . $p['end'] : '') . " (expr:{$p['code']})";
        }
    }

    private function text(mixed $text, string $path): void
    {
        if (is_array($text)) {
            foreach ($text as $i => $line) $this->text($line, "$path/$i");
            return;
        }
        if (!is_string($text) || !preg_match('/[{}\\\\]/', $text)) return;
        foreach (Analyser::analyseText($text)['errors'] as $p) {
            $blocks = $p['code'] === 'old-clock-syntax' || ($p['severity'] === 'error' && !in_array($p['code'], self::DOUBTS, true));
            if (!$blocks) continue;
            $message = $p['code'] === 'old-clock-syntax'
                ? '`{' . ($p['args'][0] ?? '') . ':clock}` is the old clock form – write `{clock(' . ($p['args'][0] ?? '') . ')}`'
                : $p['message'];
            $this->out[] = "$path: $message – at " . ($p['start'] + 1) . ($p['end'] > $p['start'] + 1 ? '–' . $p['end'] : '') . " (expr:{$p['code']})";
        }
    }
}
