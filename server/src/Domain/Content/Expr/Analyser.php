<?php
declare(strict_types=1);

namespace Scutoons\Domain\Content\Expr;

/**
 * Expressions in texts (agents/EXPRESSIONS.md §4): the server's check of a text's `{…}` – the port of @scutoons/content
 * `src/expr/analyse.ts` (same codes, positions, messages and reads; both tested against
 * shared/content/test/fixtures/expr/*.json). Parse + analyse only: the server never evaluates.
 *
 * @phpstan-type Diagnostic array{code: string, severity: string, start: int, end: int, args?: list<string>, suggestion?: string, message: string}
 * @phpstan-type Reads array{names: list<list<string>>, functions: list<array{name: string, literalArgs: list<string|int|float|bool|null>}>, sequences: int}
 * @phpstan-type Analysis array{errors: list<Diagnostic>, reads: Reads, constructs: int}
 */
final class Analyser
{
    private const MESSAGES = [
        'missing-close' => 'missing `}`',
        'missing-paren' => 'missing `)`',
        'empty-braces' => 'empty `{}` – to show a brace, write `\\{`',
        'unexpected' => 'unexpected `{0}`',
        'unexpected-char' => 'unexpected character `{0}`',
        'expected-value' => 'expected a value',
        'single-equals' => '`=` doesn’t compare – write `==`',
        'unsupported-operator' => '`{0}` isn’t supported (ink lists aren’t)',
        'unterminated-string' => 'missing closing `"`',
        'unsupported-list' => '`{0}` isn’t a list type – use `&` (cycle), `~` (shuffle), `!` (once only), `~!` or `~$`',
        'conditional-too-many' => 'conditional text has one `|` at most: `{condition: then|else}`',
        'stray-close' => 'unmatched `}` – to show it, write `\\}`',
        'too-long' => 'expression too long (more than 500 characters)',
        'too-deep' => 'nested too deeply (more than 32 levels)',
        'too-many-nodes' => 'expression too complex (more than 64 parts)',
        'too-many-constructs' => 'too many `{…}` in one text (more than {0})',
        'text-too-long' => 'text too long for `{…}` (more than {0} characters)',
        'unknown-name' => 'unknown name `{0}`',
        'unknown-variable' => 'unknown variable `{0}`',
        'unknown-function' => 'unknown function `{0}`',
        'function-needs-call' => '`{0}` is a function – write `{0}(…)`',
        'not-a-function' => '`{0}` isn’t a function – write `{0}` without `(…)`',
        'namespace-needs-member' => '`{0}` needs a name after it, e.g. `{1}`',
        'argument-count' => '`{0}` expects {1}',
        'argument-type' => '`{0}` expects a {1} as argument {2}',
        'old-clock-syntax' => '`{{0}:clock}` is the old form – write `{clock({0})}`',
        'or-in-list' => '`||` here splits a list (as in ink) – for “or”, write `or`',
        'not-in-list' => '`{!…}` is a once-only list (as in ink) – for “not”, write `not`',
        'missing-value' => '`{0}` has no value',
        'not-a-number' => '`{0}` needs numbers',
        'compare-text' => '`{0}` can’t compare texts – only `==` and `!=` can',
        'division-by-zero' => 'division by zero',
        'bad-arguments' => '`{0}` can’t use these arguments',
    ];

    /** @var list<Diagnostic> */
    private array $errors = [];
    /** @var array<string, list<string>> */
    private array $names = [];
    /** @var array<string, array{name: string, literalArgs: list<string|int|float|bool|null>}> */
    private array $functions = [];
    /** @var array<string, array{name: string, params: list<array{name: string, type: string}>, variadic: bool, result: string, state: bool}> */
    private array $fns;
    /** @var array<string, list<array{name: string, kind: string, type: string, params: list<array{name: string, type: string}>}>> */
    private array $spaces;
    /** @var array<string, int>|null */
    private ?array $known;
    private string $utf16 = '';

    /** @param list<string>|null $knownNames */
    private function __construct(?array $knownNames, string $text = '')
    {
        $this->fns = Registry::functions();
        $this->spaces = Registry::namespaces();
        $this->known = $knownNames !== null ? array_flip($knownNames) : null;
        $this->utf16 = (string) mb_convert_encoding($text, 'UTF-16LE', 'UTF-8');
    }

    /**
     * A text's problems (sorted by position) and reads.
     *
     * @param list<string>|null $knownNames the story variables there are: an unknown one becomes a warning
     * @return Analysis
     */
    public static function analyseText(string $text, ?array $knownNames = null): array
    {
        $parsed = Parser::parseText($text);
        $a = new self($knownNames, $text);
        foreach ($parsed['problems'] as $p) $a->add($p);
        foreach ($parsed['nodes'] as $n) $a->node($n);
        usort($a->errors, fn (array $x, array $y): int => [$x['start'], $x['end']] <=> [$y['start'], $y['end']]);
        return $a->result($parsed['constructs'], $parsed['lists']);
    }

    /**
     * One expression's problems and reads (event conditions).
     *
     * @param list<string>|null $knownNames
     * @return Analysis
     */
    public static function analyseExpression(string $source, ?array $knownNames = null): array
    {
        $parsed = Parser::parseExpression($source);
        $a = new self($knownNames);
        foreach ($parsed['problems'] as $p) $a->add($p);
        if ($parsed['expr'] !== null) $a->expr($parsed['expr']);
        return $a->result(0, 0);
    }

    /**
     * A problem's English message ("unknown name `map.totl` – did you mean `map.total`?").
     *
     * @param array{code: string, args?: list<string>, suggestion?: string} $p
     */
    public static function message(array $p): string
    {
        $args = $p['args'] ?? [];
        $text = (string) preg_replace_callback('/\{(\d)\}/', fn (array $m): string => $args[(int) $m[1]] ?? '', self::MESSAGES[$p['code']] ?? $p['code']);
        return isset($p['suggestion']) ? $text . ' – did you mean `' . $p['suggestion'] . '`?' : $text;
    }

    /**
     * The closest candidate, when it's close enough (at most 2 edits and a third of the word – none under 3 letters).
     *
     * @param iterable<string> $candidates
     */
    public static function didYouMean(string $word, iterable $candidates): ?string
    {
        $limit = min(2, intdiv(self::length($word), 3));
        if ($limit === 0) return null;
        $best = null;
        $bestD = PHP_INT_MAX;
        foreach ($candidates as $c) {
            if (abs(self::length($c) - self::length($word)) > $limit) continue;
            $d = self::distance(mb_strtolower($word), mb_strtolower($c));
            if ($d <= $limit && $d < $bestD) {
                $best = $c;
                $bestD = $d;
            }
        }
        return $best;
    }

    /** a string's length in UTF-16 units (as JS counts) */
    private static function length(string $s): int
    {
        return intdiv(strlen((string) mb_convert_encoding($s, 'UTF-16LE', 'UTF-8')), 2);
    }

    /** the edit distance of two words (Levenshtein, per character) */
    private static function distance(string $a, string $b): int
    {
        $x = mb_str_split($a);
        $y = mb_str_split($b);
        $prev = range(0, count($y));
        foreach ($x as $i => $cx) {
            $row = [$i + 1];
            foreach ($y as $j => $cy) $row[$j + 1] = min($prev[$j + 1] + 1, $row[$j] + 1, $prev[$j] + ($cx === $cy ? 0 : 1));
            $prev = $row;
        }
        return $prev[count($y)];
    }

    /** @param array{code: string, severity: string, start: int, end: int, args?: list<string>, suggestion?: string} $p */
    private function add(array $p): void
    {
        $this->errors[] = $p + ['message' => self::message($p)];
    }

    /**
     * @param array<string, mixed> $at a node with start / end
     * @param list<string> $args
     */
    private function problem(string $code, array $at, array $args, ?string $suggestion = null, string $severity = 'error'): void
    {
        $this->add(['code' => $code, 'severity' => $severity, 'start' => (int) $at['start'], 'end' => (int) $at['end'], 'args' => $args] + ($suggestion !== null ? ['suggestion' => $suggestion] : []));
    }

    /** @return list<string> every namespace member of a kind as `ns.member` */
    private function paths(string $kind): array
    {
        $all = [];
        foreach ($this->spaces as $ns => $members) {
            foreach ($members as $m) if ($m['kind'] === $kind) $all[] = $ns . '.' . $m['name'];
        }
        return $all;
    }

    /** @return array{0: string|null, 1: array{name: string, kind: string, type: string, params: list<array{name: string, type: string}>}|null} namespace, member */
    private function member(string $dotted): array
    {
        $dot = strpos($dotted, '.');
        if ($dot === false) return [null, null];
        $ns = substr($dotted, 0, $dot);
        if (!isset($this->spaces[$ns])) return [null, null];
        foreach ($this->spaces[$ns] as $m) if ($m['name'] === substr($dotted, $dot + 1)) return [$ns, $m];
        return [$ns, null];
    }

    /** @return list<string> */
    private function membersOf(string $ns, string $kind): array
    {
        $all = [];
        foreach ($this->spaces[$ns] ?? [] as $m) if ($m['kind'] === $kind) $all[] = $ns . '.' . $m['name'];
        return $all;
    }

    /** @param array<string, mixed> $e */
    private function expr(array $e): void
    {
        switch ($e['type']) {
            case 'name':
                $this->name($e);
                return;
            case 'call':
                /** @var list<array<string, mixed>> $args */
                $args = $e['args'];
                foreach ($args as $a) $this->expr($a);
                $this->call($e);
                return;
            case 'unary':
                /** @var array<string, mixed> $arg */
                $arg = $e['arg'];
                $this->expr($arg);
                return;
            case 'binary':
                /** @var array<string, mixed> $left */
                $left = $e['left'];
                /** @var array<string, mixed> $right */
                $right = $e['right'];
                $this->expr($left);
                $this->expr($right);
                return;
        }
    }

    /** @param array<string, mixed> $e */
    private function name(array $e): void
    {
        /** @var list<string> $path */
        $path = $e['path'];
        $full = implode('.', $path);
        if (count($path) === 1) {
            if (isset($this->spaces[$full])) {
                $this->problem('namespace-needs-member', $e, [$full, $full . '.' . ($this->spaces[$full][0]['name'] ?? 'name')]);
                return;
            }
            if (isset($this->fns[$full])) {
                $this->problem('function-needs-call', $e, [$full]);
                return;
            }
            if ($this->known !== null && !isset($this->known[$full])) {
                $this->problem('unknown-variable', $e, [$full], self::didYouMean($full, array_map('strval', array_keys($this->known))), 'warning');
            }
        } else {
            [$ns, $m] = $this->member($full);
            if ($m !== null && $m['kind'] === 'function') {
                $this->problem('function-needs-call', $e, [$full]);
                return;
            }
            if ($m === null) {
                $this->problem('unknown-name', $e, [$full], self::didYouMean($full, $ns !== null ? $this->membersOf($ns, 'value') : $this->paths('value')));
                return;
            }
        }
        if (!array_key_exists($full, $this->names)) $this->names[$full] = $path;
    }

    /** @param array<string, mixed> $e */
    private function call(array $e): void
    {
        $name = (string) $e['name'];
        /** @var list<array<string, mixed>> $args */
        $args = $e['args'];
        $fn = $this->fns[$name] ?? null;
        if ($fn !== null) {
            $params = $fn['params'];
            $variadic = $fn['variadic'];
        } else {
            [$ns, $m] = $this->member($name);
            if ($m !== null && $m['kind'] === 'value') {
                $this->problem('not-a-function', $e, [$name]);
                return;
            }
            if ($m === null) {
                $all = $ns !== null ? $this->membersOf($ns, 'function') : [...array_map('strval', array_keys($this->fns)), ...$this->paths('function')];
                $this->problem('unknown-function', $e, [$name], self::didYouMean($name, $all));
                return;
            }
            $params = $m['params'];
            $variadic = false;
        }
        $count = count($args);
        $want = count($params);
        if ($variadic ? $count < $want : $count !== $want) {
            $this->problem('argument-count', $e, [$name, ($variadic ? 'at least ' : '') . $want . ' argument' . ($want === 1 ? '' : 's')]);
            return;
        }
        foreach ($args as $i => $arg) {
            $type = ($params[$i] ?? $params[$want - 1] ?? null)['type'] ?? null;
            if ($type === 'number' && ($arg['type'] === 'str' || $arg['type'] === 'bool')) $this->problem('argument-type', $arg, [$name, 'number', (string) ($i + 1)]);
        }
        $literalArgs = [];
        foreach ($args as $arg) {
            /** @var string|int|float|bool|null $value */
            $value = in_array($arg['type'], ['num', 'str', 'bool'], true) ? $arg['value'] : null;
            $literalArgs[] = $value;
        }
        $key = (string) json_encode([$name, $literalArgs]);
        if (!isset($this->functions[$key])) $this->functions[$key] = ['name' => $name, 'literalArgs' => $literalArgs];
    }

    /** @param array<string, mixed>|null $c */
    private function content(?array $c): void
    {
        /** @var list<array<string, mixed>> $nodes */
        $nodes = $c['nodes'] ?? [];
        foreach ($nodes as $n) $this->node($n);
    }

    /** @param array<string, mixed> $n */
    private function node(array $n): void
    {
        switch ($n['type']) {
            case 'print':
                /** @var array<string, mixed> $e */
                $e = $n['expr'];
                $this->expr($e);
                return;
            case 'cond':
                /** @var array<string, mixed> $cond */
                $cond = $n['cond'];
                /** @var array{nodes: list<array<string, mixed>>} $then */
                $then = $n['then'];
                /** @var array<string, mixed>|null $else */
                $else = $n['else'];
                $this->expr($cond);
                $only = $then['nodes'][0] ?? null;
                if ($cond['type'] === 'name' && $else === null && count($then['nodes']) === 1 && $only !== null && $only['type'] === 'text' && $only['value'] === 'clock') {
                    /** @var list<string> $path */
                    $path = $cond['path'];
                    $this->problem('old-clock-syntax', $n, [implode('.', $path)], null, 'warning');
                }
                $this->content($then);
                $this->content($else);
                return;
            case 'list':
                /** @var list<array{start: int, nodes: list<array<string, mixed>>}> $items */
                $items = $n['items'];
                if ($n['kind'] === 'sequence') {
                    foreach ($items as $c) {
                        if ($c['nodes'] === [] && substr($this->utf16, ($c['start'] - 1) * 2, 4) === "|\0|\0") {
                            $this->problem('or-in-list', $n, [], null, 'warning');
                            break;
                        }
                    }
                }
                if ($n['kind'] === 'once' && count($items) === 1) $this->problem('not-in-list', $n, [], null, 'warning');
                foreach ($items as $c) $this->content($c);
                return;
        }
    }

    /** @return Analysis */
    private function result(int $constructs, int $sequences): array
    {
        return [
            'errors' => $this->errors,
            'reads' => ['names' => array_values($this->names), 'functions' => array_values($this->functions), 'sequences' => $sequences],
            'constructs' => $constructs,
        ];
    }
}
