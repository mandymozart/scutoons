<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content\Expr;

/**
 * Expressions in texts (agents/EXPRESSIONS.md): the parser of ink's inline syntax – the port of
 * @scutoons/content `src/expr/parse.ts`, rule for rule (both are tested against shared/content/test/fixtures/expr/*.json).
 * A text is literal runs and `{…}` constructs: an explicit list mark (`& ~ ! $`) → a list; an expression followed by `:`
 * → a conditional; two or more `|`-separated elements → a sequence; otherwise one expression → print. Expressions: a
 * Pratt parser with ink's operator table. Never throws; positions are UTF-16 offsets (what the creator counts), the
 * nodes are arrays shaped like the TS AST.
 *
 * @phpstan-type Node array<string, mixed>
 * @phpstan-type Problem array{code: string, severity: string, start: int, end: int, args?: list<string>}
 * @phpstan-type Tok array{k: string, v: string, start: int, end: int}
 */
final class Parser
{
    public const LENGTH = 500;
    public const DEPTH = 32;
    public const NODES = 64;
    public const CONSTRUCTS = 50;
    public const TEXT = 10000;
    public const MAX_PROBLEMS = 100;

    private const OPS2 = ['==', '!=', '<=', '>=', '&&', '||'];
    private const OPS1 = '<>+-*/%!';
    private const PUNCT = '(),:|}{';
    private const WORD_OPS = ['and', 'or', 'not', 'mod'];
    private const PREC = ['||' => 1, 'or' => 1, '&&' => 1, 'and' => 1, '==' => 2, '!=' => 2, '<' => 2, '<=' => 2, '>' => 2, '>=' => 2, '+' => 4, '-' => 5, '*' => 6, '/' => 7, '%' => 8, 'mod' => 8];
    private const NORM = ['or' => '||', 'and' => '&&', 'mod' => '%'];
    private const KINDS = ['$' => 'sequence', '&' => 'cycle', '~' => 'shuffle', '!' => 'once', '~!' => 'shuffle-once', '!~' => 'shuffle-once', '~$' => 'shuffle-stopping', '$~' => 'shuffle-stopping'];

    /** @var list<string> the text's code points */
    private array $c;
    private int $n;
    /** @var list<int> UTF-16 offset of each code point index (0 … n) */
    private array $u;
    private int $i = 0;
    /** @var list<Problem> */
    private array $problems = [];
    private int $constructs = 0;
    private int $lists = 0;
    // one expression's state
    private int $pos = 0;
    private int $from = 0;
    private int $count = 0;
    /** @var Tok|null */
    private ?array $peeked = null;

    private function __construct(string $text)
    {
        $this->c = $text === '' ? [] : (preg_split('//u', $text, -1, PREG_SPLIT_NO_EMPTY) ?: []);
        $this->n = count($this->c);
        $this->u = [0];
        $at = 0;
        foreach ($this->c as $ch) {
            $at += strlen($ch) === 4 ? 2 : 1;
            $this->u[] = $at;
        }
    }

    /**
     * A text → its nodes, problems, constructs and lists (escapes: a backslash keeps the next character).
     *
     * @return array{nodes: list<Node>, problems: list<Problem>, constructs: int, lists: int}
     */
    public static function parseText(string $text): array
    {
        $p = new self($text);
        if ($p->u[$p->n] > self::TEXT && str_contains($text, '{')) {
            $p->problem('text-too-long', 0, $p->n, [(string) self::TEXT]);
            return ['nodes' => [['type' => 'bad', 'start' => 0, 'end' => $p->u[$p->n]]], 'problems' => $p->problems, 'constructs' => 0, 'lists' => 0];
        }
        $top = $p->content(false, 0);
        /** @var list<Node> $nodes */
        $nodes = $top['nodes'];
        return ['nodes' => $nodes, 'problems' => $p->problems, 'constructs' => $p->constructs, 'lists' => $p->lists];
    }

    /**
     * A whole source as one expression (event conditions): `expr` null when it doesn't parse.
     *
     * @return array{expr: Node|null, problems: list<Problem>}
     */
    public static function parseExpression(string $source): array
    {
        $p = new self($source);
        try {
            [$expr, $end] = $p->exprAt(0);
            $t = $p->lex($end);
            if ($t['k'] !== 'eof') $p->fail('unexpected', $t['start'], $t['end'], [$p->slice($t['start'], $t['end'])]);
            return ['expr' => $expr, 'problems' => []];
        } catch (ParseFail $f) {
            $p->problem($f->problem, $f->from, $f->to, $f->args);
            return ['expr' => null, 'problems' => $p->problems];
        }
    }

    // ---- lexer -------------------------------------------------------------------------------------------------

    private function ch(int $i): string
    {
        return $this->c[$i] ?? '';
    }

    private function slice(int $from, int $to): string
    {
        return implode('', array_slice($this->c, $from, $to - $from));
    }

    private static function isDigit(string $ch): bool
    {
        return $ch !== '' && $ch >= '0' && $ch <= '9' && strlen($ch) === 1;
    }

    private static function isIdStart(string $ch): bool
    {
        return $ch !== '' && preg_match('/^[\p{L}_]$/u', $ch) === 1;
    }

    private static function isIdPart(string $ch): bool
    {
        return $ch !== '' && preg_match('/^[\p{L}\p{N}_]$/u', $ch) === 1;
    }

    private function skipWs(int $i): int
    {
        while ($i < $this->n && in_array($this->c[$i], [' ', "\t", "\r", "\n"], true)) $i++;
        return $i;
    }

    /** @param list<string>|null $args */
    private function fail(string $code, int $from, int $to, ?array $args = null): never
    {
        throw new ParseFail($code, $from, $to, $args);
    }

    private function identEnd(int $i): int
    {
        $i++;
        while (self::isIdPart($this->ch($i))) $i++;
        return $i;
    }

    /** @return Tok the next token at `i` (whitespace skipped) */
    private function lex(int $i): array
    {
        $i = $this->skipWs($i);
        if ($i >= $this->n) return ['k' => 'eof', 'v' => '', 'start' => $i, 'end' => $i];
        $ch = $this->c[$i];
        if (self::isDigit($ch)) {
            $j = $i;
            while (self::isDigit($this->ch($j))) $j++;
            if ($this->ch($j) === '.' && self::isDigit($this->ch($j + 1))) {
                $j++;
                while (self::isDigit($this->ch($j))) $j++;
            }
            return ['k' => 'num', 'v' => $this->slice($i, $j), 'start' => $i, 'end' => $j];
        }
        if (self::isIdStart($ch)) {
            $j = $this->identEnd($i);
            // a dotted name: `map.total` – the dot only joins two identifiers, no spaces
            while ($this->ch($j) === '.' && self::isIdStart($this->ch($j + 1))) $j = $this->identEnd($j + 1);
            $v = $this->slice($i, $j);
            return ['k' => in_array($v, self::WORD_OPS, true) ? 'op' : 'id', 'v' => $v, 'start' => $i, 'end' => $j];
        }
        if ($ch === '"') {
            $v = '';
            for ($j = $i + 1; $j < $this->n; $j++) {
                $x = $this->c[$j];
                if ($x === '"') return ['k' => 'str', 'v' => $v, 'start' => $i, 'end' => $j + 1];
                if ($x === '\\' && $j + 1 < $this->n) $v .= $this->c[++$j];
                else $v .= $x;
            }
            $this->fail('unterminated-string', $i, $this->n);
        }
        $two = $ch . $this->ch($i + 1);
        if (in_array($two, self::OPS2, true)) return ['k' => 'op', 'v' => $two, 'start' => $i, 'end' => $i + 2];
        if (strlen($ch) === 1 && str_contains(self::OPS1, $ch)) return ['k' => 'op', 'v' => $ch, 'start' => $i, 'end' => $i + 1];
        if (strlen($ch) === 1 && str_contains(self::PUNCT, $ch)) return ['k' => 'p', 'v' => $ch, 'start' => $i, 'end' => $i + 1];
        if ($ch === '=') $this->fail('single-equals', $i, $i + 1);
        if ($ch === '?' || $ch === '^') $this->fail('unsupported-operator', $i, $i + 1, [$ch]);
        $this->fail('unexpected-char', $i, $i + 1, [$ch]);
    }

    // ---- expressions -------------------------------------------------------------------------------------------

    /** @return Tok */
    private function peek(): array
    {
        return $this->peeked ??= $this->lex($this->pos);
    }

    /** @return Tok */
    private function next(): array
    {
        $t = $this->peek();
        if ($this->u[$t['end']] - $this->u[$this->from] > self::LENGTH) $this->fail('too-long', $this->from, $t['end']);
        $this->pos = $t['end'];
        $this->peeked = null;
        return $t;
    }

    /**
     * @param Node $n a node with code point positions → counted, positions as UTF-16
     * @return Node
     */
    private function node(array $n, int $start, int $end): array
    {
        if (++$this->count > self::NODES) $this->fail('too-many-nodes', $start, $end);
        return $n + ['start' => $this->u[$start], 'end' => $this->u[$end], 'cs' => $start, 'ce' => $end];
    }

    private function deep(int $d): void
    {
        if ($d > self::DEPTH) {
            $t = $this->peek();
            $this->fail('too-deep', $t['start'], $t['end']);
        }
    }

    /** @return array{0: Node, 1: int} one expression from `start`, and where it ends (code points) */
    private function exprAt(int $start): array
    {
        $this->pos = $start;
        $this->peeked = null;
        $this->count = 0;
        $this->from = $this->skipWs($start);
        $e = $this->expr(0, 1);
        return [$e, $this->pos];
    }

    /** @return Node */
    private function expr(int $min, int $d): array
    {
        $this->deep($d);
        $left = $this->unary($d);
        for (;;) {
            $t = $this->peek();
            $p = $t['k'] === 'op' ? (self::PREC[$t['v']] ?? 0) : 0;
            if ($p === 0 || $p <= $min) return $left;
            $this->next();
            $right = $this->expr($p, $d + 1);
            $left = $this->node(['type' => 'binary', 'op' => self::NORM[$t['v']] ?? $t['v'], 'left' => $left, 'right' => $right], (int) $left['cs'], (int) $right['ce']);
        }
    }

    /** @return Node */
    private function unary(int $d): array
    {
        $t = $this->peek();
        if ($t['k'] === 'op' && in_array($t['v'], ['-', '!', 'not'], true)) {
            $this->deep($d + 1);
            $this->next();
            $arg = $this->unary($d + 1);
            return $this->node(['type' => 'unary', 'op' => $t['v'] === '-' ? '-' : '!', 'arg' => $arg], $t['start'], (int) $arg['ce']);
        }
        return $this->primary($d);
    }

    private static function number(string $v): int|float
    {
        // as a JS number: whole values are ints (JSON `2`, not `2.0`)
        $f = (float) $v;
        return $f === floor($f) && abs($f) < 2 ** 53 ? (int) $f : $f;
    }

    /** @return Node */
    private function primary(int $d): array
    {
        $t = $this->next();
        if ($t['k'] === 'num') return $this->node(['type' => 'num', 'value' => self::number($t['v'])], $t['start'], $t['end']);
        if ($t['k'] === 'str') return $this->node(['type' => 'str', 'value' => $t['v']], $t['start'], $t['end']);
        if ($t['k'] === 'id') {
            if ($t['v'] === 'true' || $t['v'] === 'false') return $this->node(['type' => 'bool', 'value' => $t['v'] === 'true'], $t['start'], $t['end']);
            if ($this->peek()['v'] === '(' && $this->peek()['k'] === 'p') {
                $this->next();
                $args = [];
                $close = $this->peek();
                if ($close['k'] === 'p' && $close['v'] === ')') $this->next();
                else {
                    for (;;) {
                        $this->deep($d + 1);
                        $args[] = $this->expr(0, $d + 1);
                        $close = $this->next();
                        if ($close['k'] === 'p' && $close['v'] === ',') continue;
                        if ($close['k'] === 'p' && $close['v'] === ')') break;
                        $this->fail($close['k'] === 'eof' || $close['k'] === 'p' ? 'missing-paren' : 'unexpected', $close['start'], $close['end'], [$this->slice($close['start'], $close['end'])]);
                    }
                }
                return $this->node(['type' => 'call', 'name' => $t['v'], 'args' => $args], $t['start'], $close['end']);
            }
            return $this->node(['type' => 'name', 'path' => explode('.', $t['v'])], $t['start'], $t['end']);
        }
        if ($t['k'] === 'p' && $t['v'] === '(') {
            $this->deep($d + 1);
            $inner = $this->expr(0, $d + 1);
            $close = $this->next();
            if ($close['k'] !== 'p' || $close['v'] !== ')') {
                $this->fail($close['k'] === 'eof' || $close['k'] === 'p' ? 'missing-paren' : 'unexpected', $close['start'], $close['end'], [$this->slice($close['start'], $close['end'])]);
            }
            return $inner;
        }
        if ($t['k'] === 'eof' || ($t['k'] === 'p' && $t['v'] !== '{')) $this->fail('expected-value', $t['start'], $t['end']);
        $this->fail('unexpected', $t['start'], $t['end'], [$this->slice($t['start'], $t['end'])]);
    }

    // ---- text --------------------------------------------------------------------------------------------------

    /** @param list<string>|null $args (positions: code points) */
    private function problem(string $code, int $from, int $to, ?array $args = null, string $severity = 'error'): void
    {
        if (count($this->problems) >= self::MAX_PROBLEMS) return;
        $this->problems[] = ['code' => $code, 'severity' => $severity, 'start' => $this->u[$from], 'end' => $this->u[$to]] + ($args !== null ? ['args' => $args] : []);
    }

    /** @return Node literal text and constructs up to the end – or, inside a construct, up to its `|` / `}` */
    private function content(bool $inside, int $depth): array
    {
        $start = $this->i;
        $nodes = [];
        $buf = '';
        $from = $this->i;
        while ($this->i < $this->n) {
            $ch = $this->c[$this->i];
            if ($buf === '') $from = $this->i;
            if ($ch === '\\') {
                $buf .= $this->i + 1 < $this->n ? $this->c[$this->i + 1] : '\\';
                $this->i = min($this->n, $this->i + 2);
            } elseif ($ch === '{') {
                if ($buf !== '') $nodes[] = $this->text($buf, $from);
                $buf = '';
                $nodes[] = $this->construct($depth);
            } elseif ($ch === '}' && !$inside) {
                $this->problem('stray-close', $this->i, $this->i + 1, null, 'warning');
                $buf .= $ch;
                $this->i++;
            } elseif (($ch === '}' || $ch === '|') && $inside) {
                break;
            } else {
                $buf .= $ch;
                $this->i++;
            }
        }
        if ($buf !== '') $nodes[] = $this->text($buf, $from);
        return ['start' => $this->u[$start], 'end' => $this->u[$this->i], 'nodes' => $nodes];
    }

    /** @return Node a literal run from `from` up to here */
    private function text(string $value, int $from): array
    {
        return ['type' => 'text', 'value' => $value, 'start' => $this->u[$from], 'end' => $this->u[$this->i]];
    }

    /** @return array{items: list<Node>, closed: bool} */
    private function items(int $depth): array
    {
        $all = [$this->content(true, $depth + 1)];
        while ($this->ch($this->i) === '|') {
            $this->i++;
            $all[] = $this->content(true, $depth + 1);
        }
        $closed = $this->ch($this->i) === '}';
        if ($closed) $this->i++;
        return ['items' => $all, 'closed' => $closed];
    }

    private function skipToClose(int $from): int
    {
        $level = 0;
        for ($j = $from; $j < $this->n; $j++) {
            $ch = $this->c[$j];
            if ($ch === '\\') $j++;
            elseif ($ch === '{') $level++;
            elseif ($ch === '}' && $level-- === 0) return $j + 1;
        }
        return $this->n;
    }

    /** @return Node */
    private function bad(int $start): array
    {
        return ['type' => 'bad', 'start' => $this->u[$start], 'end' => $this->u[$this->i]];
    }

    /** @return Node one `{…}` at `i` */
    private function construct(int $depth): array
    {
        $s = $this->i;
        if (++$this->constructs > self::CONSTRUCTS) {
            $this->problem('too-many-constructs', $s, $this->n, [(string) self::CONSTRUCTS]);
            $this->i = $this->n;
            return $this->bad($s);
        }
        if ($depth >= self::DEPTH) {
            $this->i = $this->skipToClose($s + 1);
            $this->problem('too-deep', $s, $this->i, [(string) self::DEPTH]);
            return $this->bad($s);
        }
        $this->i = $this->skipWs($s + 1);
        $mark = '';
        while (in_array($this->ch($this->i), ['!', '&', '~', '$'], true) && strlen($mark) < 3) $mark .= $this->c[$this->i++];
        if ($mark !== '') {
            $kind = self::KINDS[$mark] ?? null;
            $index = $this->lists++;
            $r = $this->items($depth);
            if (!$r['closed']) {
                $this->problem('missing-close', $s, $this->n);
                return $this->bad($s);
            }
            if ($kind === null) {
                $this->problem('unsupported-list', $s, $s + 1 + strlen($mark), [$mark]);
                return $this->bad($s);
            }
            return ['type' => 'list', 'kind' => $kind, 'index' => $index, 'items' => $r['items'], 'start' => $this->u[$s], 'end' => $this->u[$this->i]];
        }
        $p = $this->i;
        $attempt = null;
        $failure = null;
        try {
            $attempt = $this->exprAt($p);
        } catch (ParseFail $f) {
            $failure = $f;
        }
        $q = $attempt !== null ? $this->skipWs($attempt[1]) : -1;
        if ($attempt !== null && $this->ch($q) === ':') {
            $this->i = $q + 1;
            $then = $this->content(true, $depth + 1);
            $else = null;
            if ($this->ch($this->i) === '|') {
                $this->i++;
                $else = $this->content(true, $depth + 1);
                if ($this->ch($this->i) === '|') {
                    $this->problem('conditional-too-many', $this->i, $this->i + 1);
                    while ($this->ch($this->i) === '|') {
                        $this->i++;
                        $this->content(true, $depth + 1);
                    }
                    if ($this->ch($this->i) === '}') $this->i++;
                    else $this->problem('missing-close', $s, $this->n);
                    return $this->bad($s);
                }
            }
            if ($this->ch($this->i) !== '}') {
                $this->problem('missing-close', $s, $this->n);
                return $this->bad($s);
            }
            $this->i++;
            return ['type' => 'cond', 'cond' => $attempt[0], 'then' => $then, 'else' => $else, 'start' => $this->u[$s], 'end' => $this->u[$this->i]];
        }
        // a sequence when there are two or more elements – otherwise its parse is thrown away again
        $before = [count($this->problems), $this->constructs, $this->lists];
        $index = $this->lists++;
        $this->i = $p;
        $r = $this->items($depth);
        if ($r['closed'] && count($r['items']) >= 2) {
            return ['type' => 'list', 'kind' => 'sequence', 'index' => $index, 'items' => $r['items'], 'start' => $this->u[$s], 'end' => $this->u[$this->i]];
        }
        $this->problems = array_slice($this->problems, 0, $before[0]);
        [, $this->constructs, $this->lists] = $before;
        if ($attempt !== null && $this->ch($q) === '}') {
            $this->i = $q + 1;
            return ['type' => 'print', 'expr' => $attempt[0], 'start' => $this->u[$s], 'end' => $this->u[$this->i]];
        }
        if (!$r['closed']) $this->problem('missing-close', $s, $this->n);
        elseif (preg_match('/^[\s\p{Z}\x{FEFF}]*$/u', $this->slice($s + 1, $this->i - 1)) === 1) $this->problem('empty-braces', $s, $this->i);
        elseif ($failure !== null) $this->problem($failure->problem, $failure->from, $failure->to, $failure->args);
        else {
            try {
                $t = $this->lex($q);
                $this->problem('unexpected', $t['start'], $t['end'], [$this->slice($t['start'], $t['end'])]);
            } catch (ParseFail $f) {
                $this->problem($f->problem, $f->from, $f->to, $f->args);
            }
        }
        return $this->bad($s);
    }
}
