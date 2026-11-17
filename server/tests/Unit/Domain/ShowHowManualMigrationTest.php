<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The show how-manual migration (2026-11-16-show-how-manual): a row's `fire.how` "hand" becomes "manual" in every stored
 * show version, compact or spaced; auto / suggest rows, other kinds and a second run stay as they are.
 */
final class ShowHowManualMigrationTest extends TestCase
{
    private const KEYS = "('show:test-hand', 'show:test-spaced', 'scene:test-hand')";

    private function insert(string $key, string $kind, string $json): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, '2026-11-16 00:00:00', 'draft', NULL)",
            [$key, $kind, $json]);
    }

    /** @return array<string, string> */
    private function rows(): array
    {
        $out = [];
        foreach ($this->db->all('SELECT entity_key, json FROM entity_versions WHERE entity_key IN ' . self::KEYS . ' ORDER BY id') as $r) {
            $out[(string) $r['entity_key']] = (string) $r['json'];
        }
        return $out;
    }

    public function testHandBecomesManualInShowsOnly(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $show = json_encode(['id' => 'test-hand', 'rows' => [
            ['id' => 'a', 'fire' => ['event' => 'door', 'how' => 'hand']],
            ['id' => 'b', 'fire' => ['event' => 'roar', 'how' => 'suggest']],
        ]], JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        $this->insert('show:test-hand', 'show', $show);
        $this->insert('show:test-spaced', 'show', '{"id": "test-spaced", "rows": [{"id": "a", "fire": {"how": "hand"}}]}');
        $this->insert('scene:test-hand', 'scene', '{"how":"hand"}');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-16-show-how-manual.mysql.sql');
        self::assertCount(1, $statements);
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->rows();

        $rows = json_decode($after['show:test-hand'], true)['rows'];
        self::assertSame(['manual', 'suggest'], array_map(fn(array $r) => $r['fire']['how'], $rows));
        self::assertSame('manual', json_decode($after['show:test-spaced'], true)['rows'][0]['fire']['how']);
        self::assertSame('{"how":"hand"}', $after['scene:test-hand'], 'another kind is left alone');

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->rows(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
