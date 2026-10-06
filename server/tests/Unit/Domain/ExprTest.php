<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Content\Expr\Analyser;
use Scutoons\Domain\Content\Expr\Parser;
use Scutoons\Domain\Content\Expr\Registry;

/**
 * Expressions in texts (agents/EXPRESSIONS.md §4): the server's parser + analyser give exactly the AST, problems (codes,
 * positions, messages) and reads of @scutoons/content `src/expr` – both read the golden fixtures
 * shared/content/test/fixtures/expr/*.json; registry.json keeps Registry equal to the TS registry.
 */
final class ExprTest extends TestCase
{
    private const FIXTURES = __DIR__ . '/../../../../shared/content/test/fixtures/expr';
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_LINE_TERMINATORS;

    public function testGoldenFixtures(): void
    {
        $files = array_values(array_filter(glob(self::FIXTURES . '/*.json') ?: [], fn (string $f): bool => basename($f) !== 'registry.json'));
        self::assertGreaterThanOrEqual(6, count($files));
        $cases = 0;
        foreach ($files as $file) {
            /** @var array{knownNames?: list<string>, cases: list<array{text: string, ast: string, errors: list<array<string, mixed>>, reads: array<string, mixed>, constructs: int}>} $fixture */
            $fixture = json_decode((string) file_get_contents($file), true, 512, JSON_THROW_ON_ERROR);
            foreach ($fixture['cases'] as $i => $case) {
                $where = basename($file) . " #$i " . json_encode($case['text'], self::JSON);
                $parsed = Parser::parseText($case['text']);
                self::assertSame($case['ast'], self::summary($parsed['nodes']), "$where: AST");
                $analysis = Analyser::analyseText($case['text'], $fixture['knownNames'] ?? null);
                $errors = array_map(fn (array $e): array => ['code' => $e['code'], 'severity' => $e['severity'], 'start' => $e['start'], 'end' => $e['end'], 'message' => $e['message']], $analysis['errors']);
                self::assertSame($case['errors'], $errors, "$where: errors");
                self::assertSame($case['reads'], $analysis['reads'], "$where: reads");
                self::assertSame($case['constructs'], $analysis['constructs'], "$where: constructs");
                $cases++;
            }
        }
        self::assertGreaterThan(150, $cases);
    }

    public function testRegistryIsTheTsRegistry(): void
    {
        $want = json_decode((string) file_get_contents(self::FIXTURES . '/registry.json'), true, 512, JSON_THROW_ON_ERROR);
        self::assertSame($want, ['functions' => Registry::FUNCTIONS, 'namespaces' => Registry::NAMESPACES]);
    }

    public function testExpressionsAndMessages(): void
    {
        $ok = Analyser::analyseExpression('coins >= 3 and inventory.has("key")', ['coins']);
        self::assertSame([], $ok['errors']);
        self::assertSame([['coins']], $ok['reads']['names']);
        self::assertSame([['name' => 'inventory.has', 'literalArgs' => ['key']]], $ok['reads']['functions']);
        self::assertSame(['expected-value'], array_column(Analyser::analyseExpression('coins >=')['errors'], 'code'));
        self::assertSame(['unexpected'], array_column(Analyser::analyseExpression('coins 3')['errors'], 'code'));
        $e = Analyser::analyseText('{map.totl}')['errors'][0];
        self::assertSame('unknown name `map.totl` – did you mean `map.total`?', $e['message']);
        self::assertSame([1, 9], [$e['start'], $e['end']]);
        self::assertSame('`clock` expects 1 argument', Analyser::analyseText('{clock()}')['errors'][0]['message']);
        self::assertSame('total', Analyser::didYouMean('totl', ['total', 'tool']));
        self::assertNull(Analyser::didYouMean('ab', ['abc']));
        // positions are UTF-16 offsets, as the creator counts them
        $astral = Analyser::analyseText('🙂🙂 {map.totl}')['errors'][0];
        self::assertSame([6, 14], [$astral['start'], $astral['end']]);
    }

    public function testLimitsAndHostileInputsStayFast(): void
    {
        $codes = fn (string $text): array => array_column(Analyser::analyseText($text)['errors'], 'code');
        self::assertContains('too-long', $codes('{"' . str_repeat('x', 600) . '"}'));
        self::assertContains('too-deep', $codes('{' . str_repeat('(', 40) . '1' . str_repeat(')', 40) . '}'));
        self::assertContains('too-many-nodes', $codes('{' . implode('+', array_fill(0, 70, '1')) . '}'));
        self::assertContains('too-many-constructs', $codes(str_repeat('{a}', 51)));
        self::assertSame([], $codes(str_repeat('{a}', 50)));
        self::assertContains('text-too-long', $codes('{a} ' . str_repeat('x', Parser::TEXT)));
        foreach ([str_repeat('{', 100000), str_repeat('}', 100000), str_repeat('{|', 50000), '{' . str_repeat('"', 100000) . '}', '{' . str_repeat('(', 100000), '{a' . str_repeat('|', 100000) . '}', '{' . str_repeat('1+', 50000) . '1}', str_repeat('\\', 100001)] as $text) {
            $start = microtime(true);
            Analyser::analyseText($text);
            self::assertLessThan(2.0, microtime(true) - $start, substr($text, 0, 20));
        }
    }

    /** @param list<array<string, mixed>> $nodes the AST as the S-expression of expr-golden.test.ts `summary` */
    private static function summary(array $nodes): string
    {
        return implode(' ', array_map(self::node(...), $nodes));
    }

    private static function json(mixed $value): string
    {
        return (string) json_encode($value, self::JSON);
    }

    /** @param array<string, mixed> $e */
    private static function expr(array $e): string
    {
        /** @var list<array<string, mixed>> $args */
        $args = $e['args'] ?? [];
        /** @var list<string> $path */
        $path = $e['path'] ?? [];
        /** @var array<string, mixed> $arg */
        $arg = $e['arg'] ?? [];
        /** @var array<string, mixed> $left */
        $left = $e['left'] ?? [];
        /** @var array<string, mixed> $right */
        $right = $e['right'] ?? [];
        return match ($e['type']) {
            'num', 'str', 'bool' => self::json($e['value']),
            'name' => implode('.', $path),
            'call' => '(call ' . self::str($e['name']) . implode('', array_map(fn (array $a): string => ' ' . self::expr($a), $args)) . ')',
            'unary' => '(' . self::str($e['op']) . ' ' . self::expr($arg) . ')',
            'binary' => '(' . self::str($e['op']) . ' ' . self::expr($left) . ' ' . self::expr($right) . ')',
            default => '?',
        };
    }

    private static function str(mixed $v): string
    {
        return is_string($v) ? $v : '';
    }

    /** @param array<string, mixed>|null $c */
    private static function content(?array $c): string
    {
        if ($c === null) return 'nil';
        /** @var list<array<string, mixed>> $nodes */
        $nodes = $c['nodes'];
        return '[' . implode(' ', array_map(self::node(...), $nodes)) . ']';
    }

    /** @param array<string, mixed> $n */
    private static function node(array $n): string
    {
        /** @var array<string, mixed> $expr */
        $expr = $n['expr'] ?? $n['cond'] ?? [];
        /** @var list<array<string, mixed>> $items */
        $items = $n['items'] ?? [];
        /** @var array<string, mixed>|null $then */
        $then = $n['then'] ?? null;
        /** @var array<string, mixed>|null $else */
        $else = $n['else'] ?? null;
        return match ($n['type']) {
            'text' => self::json($n['value']),
            'print' => '(print ' . self::expr($expr) . ')',
            'cond' => '(if ' . self::expr($expr) . ' ' . self::content($then) . ' ' . self::content($else) . ')',
            'list' => '(' . self::str($n['kind']) . '#' . self::json($n['index']) . implode('', array_map(fn (array $c): string => ' ' . self::content($c), $items)) . ')',
            'bad' => 'bad',
            default => '?',
        };
    }
}
