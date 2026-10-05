<?php
declare(strict_types=1);

namespace Scutoons\Infrastructure\Database;

use RuntimeException;

/**
 * Production's content + catalog as the seed of a local database (agents/CUTOVER.md 10.30, docs/DATABASE.md
 * "Production seed"). Production is the source of truth; the repository carries no content / catalog data any more.
 *
 * The file is a phpMyAdmin SQL export of database d0487143 (`server/db/prod-seed.sql`, gitignored – entity_versions
 * may name creators). Only the INSERTs of TABLES are used: a full dump (structure + every table) is fine too –
 * extract() keeps the allowed tables' INSERTs and drops the rest (users, sessions, qr, inventory … never leave it).
 *
 * load(): in one transaction, every table of TABLES is emptied (the migrations' own seed rows – e.g. the
 * 2026-10-04-catalog snapshot – go too) and the dump's rows are inserted. The file's SHA-256 is remembered in the
 * setting `dev_prod_seed`: loading the same file again is a no-op unless forced, so `npm run dev:db` can run any
 * time without wiping local edits.
 */
final class ProdSeed
{
    /** The tables taken from production, in insert order (parents before children) */
    public const TABLES = [
        'entity_versions',
        'toons',
        'catalog_meta',
        'catalog_items',
        'catalog_recipes',
        'catalog_albums',
        'catalog_editions',
        'catalog_products',
        'catalog_featured',
        'catalog_permissions',
        'catalog_code_types',
        'assets',
        'asset_versions',
        'asset_renditions',
        'asset_usages',
    ];
    /** settings key of the loaded file's hash */
    public const MARKER = 'dev_prod_seed';

    /**
     * The INSERT statements of the allowed tables in a phpMyAdmin dump (one row per line, every line of a statement
     * but the last ends in "," – strings never span lines: the dump escapes line breaks as \n)
     * @return array{statements: array<string, list<string>>, skipped: array<string, int>} table → statements; other tables → their statement count
     */
    public static function extract(string $sql): array
    {
        $statements = array_fill_keys(self::TABLES, []);
        $skipped = [];
        $lines = preg_split('/\r?\n/', $sql) ?: [];
        $count = count($lines);
        for ($i = 0; $i < $count; $i++) {
            if (!preg_match('/^INSERT INTO `([A-Za-z0-9_]+)`/', $lines[$i], $m)) continue;
            $start = $i;
            while ($i < $count && !str_ends_with(rtrim($lines[$i]), ';')) $i++;
            if ($i >= $count) throw new RuntimeException("Unterminated INSERT INTO `{$m[1]}` (line " . ($start + 1) . ')');
            if (isset($statements[$m[1]])) $statements[$m[1]][] = implode("\n", array_slice($lines, $start, $i - $start + 1));
            else $skipped[$m[1]] = ($skipped[$m[1]] ?? 0) + 1;
        }
        return ['statements' => $statements, 'skipped' => $skipped];
    }

    /**
     * The seed file's text for the extracted statements (only the allowed tables)
     * @param array<string, list<string>> $statements
     */
    public static function render(array $statements, string $source): string
    {
        $out = "-- SCUTOONS production seed (agents/CUTOVER.md 10.30) – gitignored, never commit.\n"
            . '-- Extracted ' . gmdate('Y-m-d H:i') . " UTC from $source: only the INSERTs of " . implode(', ', self::TABLES) . ".\n"
            . "-- Load: npm run dev:db (or npm run dev:db seed).\n\n";
        foreach ($statements as $table => $list) {
            if ($list === []) continue;
            $out .= "-- $table\n" . implode("\n", $list) . "\n\n";
        }
        return $out;
    }

    /**
     * Empties TABLES and inserts the file's rows – skipped when this file (by hash) is already loaded, unless $force
     * @return array{loaded: bool, hash: string, rows: array<string, int>, skipped: array<string, int>}
     */
    public static function load(Database $db, string $sql, bool $force = false, string $by = 'cli'): array
    {
        $hash = hash('sha256', $sql);
        $loaded = json_decode((string) ($db->value('SELECT `value` FROM settings WHERE `key` = ?', [self::MARKER]) ?? 'null'), true);
        $rows = fn() => array_combine(self::TABLES, array_map(fn(string $t) => (int) $db->value("SELECT COUNT(*) FROM `$t`"), self::TABLES));
        if (!$force && is_array($loaded) && ($loaded['sha256'] ?? null) === $hash) return ['loaded' => false, 'hash' => $hash, 'rows' => $rows(), 'skipped' => []];
        ['statements' => $statements, 'skipped' => $skipped] = self::extract($sql);
        if (array_merge(...array_values($statements)) === []) throw new RuntimeException('No INSERT of ' . implode(', ', self::TABLES) . ' in the file – is it a data export?');
        $db->exec('SET FOREIGN_KEY_CHECKS = 0');
        try {
            $db->transaction(function () use ($db, $statements, $hash, $by) {
                foreach (array_reverse(self::TABLES) as $table) $db->exec("DELETE FROM `$table`");
                foreach ($statements as $list) {
                    foreach ($list as $statement) $db->exec($statement);
                }
                $value = json_encode(['sha256' => $hash, 'loaded_at' => gmdate('Y-m-d H:i:s')]);
                $db->run('DELETE FROM settings WHERE `key` = ?', [self::MARKER]);
                $db->run('INSERT INTO settings (`key`, `value`, updated_at, updated_by) VALUES (?, ?, ?, ?)', [self::MARKER, $value, gmdate('Y-m-d H:i:s'), $by]);
            });
        } finally {
            $db->exec('SET FOREIGN_KEY_CHECKS = 1');
        }
        return ['loaded' => true, 'hash' => $hash, 'rows' => $rows(), 'skipped' => $skipped];
    }
}
