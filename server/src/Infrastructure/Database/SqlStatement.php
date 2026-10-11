<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;

/**
 * What a migration statement is (BACKLOG 10.61): `data` (INSERT / UPDATE / DELETE / REPLACE / SELECT / WITH / SET @var)
 * runs inside a transaction and can be dry-run (rolled back); `ddl` (CREATE / ALTER / DROP / RENAME / TRUNCATE) commits
 * implicitly – never dry-run, never inside a transaction; anything else (`other`: COMMIT, LOCK TABLES, SET autocommit …)
 * is not dry-run either. And which rows a statement changes, so they can be backed up right before it runs.
 */
final class SqlStatement
{
    public const DATA = 'data';
    public const DDL = 'ddl';
    public const OTHER = 'other';

    private const TABLE = '(`[^`]+`|\w+)';
    private const ALIAS = '(?:\s+(?:AS\s+)?(?!(?:SET|WHERE|ORDER|LIMIT|USING|JOIN|INNER|LEFT|RIGHT|CROSS|STRAIGHT_JOIN)\b)(\w+))?';

    public static function kind(string $sql): string
    {
        $sql = ltrim($sql, " \t\r\n(");
        if (preg_match('/^(INSERT|UPDATE|DELETE|REPLACE|SELECT|WITH)\b/i', $sql) === 1) return self::DATA;
        if (preg_match('/^SET\s+@/i', $sql) === 1) return self::DATA;
        if (preg_match('/^(CREATE|ALTER|DROP|RENAME|TRUNCATE)\b/i', $sql) === 1) return self::DDL;
        return self::OTHER;
    }

    /**
     * The SELECT that reads the rows a statement is about to change: '' = it changes none that exist (INSERT, SELECT,
     * SET), null = can't tell (UPDATE / DELETE over several tables, INSERT … ON DUPLICATE KEY UPDATE, REPLACE)
     */
    public static function backupSelect(string $sql): ?string
    {
        $sql = trim($sql);
        $mask = self::topLevel($sql);
        if (preg_match('/^WITH\b/i', $mask) === 1) return preg_match('/\b(UPDATE|DELETE|REPLACE)\b/i', $mask) === 1 ? null : '';
        if (preg_match('/^(SELECT|SET)\b/i', $mask) === 1) return '';
        if (preg_match('/^REPLACE\b/i', $mask) === 1) return null;
        if (preg_match('/^INSERT\b/i', $mask) === 1) return preg_match('/\bON\s+DUPLICATE\s+KEY\s+UPDATE\b/i', $mask) === 1 ? null : '';

        if (preg_match('/^UPDATE\s+(?:LOW_PRIORITY\s+)?(?:IGNORE\s+)?' . self::TABLE . self::ALIAS . '\s+SET\s/i', $mask, $m, PREG_OFFSET_CAPTURE) === 1) {
            $from = self::from($m[1][0], $m[2][0] ?? '');
            return 'SELECT * FROM ' . $from . self::where($sql, $mask, strlen($m[0][0]));
        }
        if (preg_match('/^DELETE\s+(?:LOW_PRIORITY\s+)?(?:QUICK\s+)?(?:IGNORE\s+)?FROM\s+' . self::TABLE . self::ALIAS . '(?=\s+WHERE\s|\s+ORDER\s|\s+LIMIT\s|\s*$)/i', $mask, $m, PREG_OFFSET_CAPTURE) === 1) {
            if (preg_match('/\b(USING|JOIN)\b/i', $mask) === 1) return null;
            $from = self::from($m[1][0], $m[2][0] ?? '');
            return 'SELECT * FROM ' . $from . self::where($sql, $mask, strlen($m[0][0]));
        }
        return null;
    }

    /** The table a backed-up row came from (for the backup file) */
    public static function table(string $select): string
    {
        return preg_match('/^SELECT \* FROM `?([^`\s]+)`?/', $select, $m) === 1 ? $m[1] : '';
    }

    private static function from(string $table, string $alias): string
    {
        $table = '`' . trim($table, '`') . '`';
        return $alias !== '' ? "$table AS `$alias`" : $table;
    }

    /** ` WHERE …` of the statement (top-level, up to ORDER BY / LIMIT), '' when it has none (every row) */
    private static function where(string $sql, string $mask, int $offset): string
    {
        if (preg_match('/\bWHERE\b/i', $mask, $w, PREG_OFFSET_CAPTURE, $offset) !== 1) return '';
        $start = $w[0][1] + 5;
        $end = strlen($sql);
        if (preg_match('/\b(ORDER\s+BY|LIMIT)\b/i', $mask, $e, PREG_OFFSET_CAPTURE, $start) === 1) $end = $e[0][1];
        return ' WHERE ' . trim(substr($sql, $start, $end - $start));
    }

    /**
     * The statement with everything inside quotes, backticks and parentheses blanked out (same length), so keywords are
     * only found at the top level – a subquery's WHERE or a string's "JOIN" don't count. Backticked names stay.
     */
    private static function topLevel(string $sql): string
    {
        $out = '';
        $depth = 0;
        $quote = null;
        $length = strlen($sql);
        for ($i = 0; $i < $length; $i++) {
            $c = $sql[$i];
            if ($quote !== null) {
                if ($c === '\\' && $quote !== '`' && $i + 1 < $length) {
                    $out .= '  ';
                    $i++;
                    continue;
                }
                $keep = $depth === 0 && ($quote === '`' || $c === $quote);
                if ($c === $quote) {
                    if ($quote !== '`' && $i + 1 < $length && $sql[$i + 1] === $quote) { // '' inside a string
                        $out .= '  ';
                        $i++;
                        continue;
                    }
                    $quote = null;
                }
                $out .= $keep ? $c : ' ';
                continue;
            }
            if ($c === "'" || $c === '"' || $c === '`') {
                $quote = $c;
                $out .= $depth === 0 ? $c : ' ';
                continue;
            }
            if ($c === '(') {
                $depth++;
                $out .= $depth === 1 ? '(' : ' ';
                continue;
            }
            if ($c === ')') {
                $depth = max(0, $depth - 1);
                $out .= $depth === 0 ? ')' : ' ';
                continue;
            }
            $out .= $depth > 0 ? ' ' : $c;
        }
        return $out;
    }
}
