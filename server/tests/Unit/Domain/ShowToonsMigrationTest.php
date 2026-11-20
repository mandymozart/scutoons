<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The show toons migration (2026-11-20-show-toons, #708): a show's one `toon` becomes the list `toons` in every stored
 * version; an empty toon an empty list, a document with `toons` keeps it; other kinds and a second run stay as they are.
 */
final class ShowToonsMigrationTest extends TestCase
{
    private const KEYS = "('show:test-toon', 'show:test-empty', 'show:test-toons', 'scene:test-toon')";

    private function insert(string $key, string $kind, string $json): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, '2026-11-20 00:00:00', 'draft', NULL)",
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

    public function testTheToonBecomesTheListInShowsOnly(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $this->insert('show:test-toon', 'show', '{"id":"test-toon","version":3,"toon":"catpope","outlets":[{"id":"left"}],"rows":[]}');
        $this->insert('show:test-toon', 'show', '{"id": "test-toon", "version": 2, "toon": "catpope", "rows": []}');
        $this->insert('show:test-empty', 'show', '{"id":"test-empty","toon":"","rows":[]}');
        $this->insert('show:test-toons', 'show', '{"id":"test-toons","toons":["catpope","yellowmouse"],"rows":[]}');
        $this->insert('scene:test-toon', 'scene', '{"toon":"catpope"}');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-20-show-toons.mysql.sql');
        self::assertCount(1, $statements);

        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->rows();

        $versions = $this->db->all("SELECT json FROM entity_versions WHERE entity_key = 'show:test-toon' ORDER BY id");
        foreach ($versions as $v) {
            $doc = json_decode((string) $v['json'], true);
            self::assertSame(['catpope'], $doc['toons'], 'every version, compact or spaced');
            self::assertArrayNotHasKey('toon', $doc);
        }
        self::assertSame([['id' => 'left']], json_decode((string) $versions[0]['json'], true)['outlets'], 'the rest stays');
        self::assertSame([], json_decode($after['show:test-empty'], true)['toons'], 'an empty toon: none');
        self::assertSame(['catpope', 'yellowmouse'], json_decode($after['show:test-toons'], true)['toons']);
        self::assertSame('{"toon":"catpope"}', $after['scene:test-toon'], 'another kind is left alone');

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->rows(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
