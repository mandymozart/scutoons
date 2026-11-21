<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The place chapter migration (2026-11-21-place-chapter, #710): every stored version of a place loses `chapter`; the
 * rest of the place, a route's `toons`, other kinds and a second run stay as they are.
 */
final class PlaceChapterMigrationTest extends TestCase
{
    private const KEYS = "('place:test-home', 'place:test-plain', 'route:test-way', 'chapter:test-toon:1')";

    private function insert(string $key, string $kind, string $json): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, '2026-11-21 00:00:00', 'draft', NULL)",
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

    public function testThePlaceLosesItsChapterLink(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $this->insert('place:test-home', 'place', '{"name":{"de":"Zuhause"},"x":1,"y":2,"toons":["catpope"],"chapter":{"toon":"catpope","number":2},"hidden":true}');
        $this->insert('place:test-home', 'place', '{"name": {"de": "Zuhause"}, "x": 1, "y": 2, "toons": ["catpope"], "chapter": {"toon": "catpope", "number": 1}}');
        $this->insert('place:test-plain', 'place', '{"name":{"de":"P"},"x":3,"y":4,"toons":[]}');
        $this->insert('route:test-way', 'route', '{"name":{"de":"Weg"},"toons":["catpope"],"waypoints":[]}');
        $this->insert('chapter:test-toon:1', 'chapter', '{"chapter":{"toon":"x","number":1}}');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-place-chapter.mysql.sql');
        self::assertCount(1, $statements);
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->rows();

        foreach ($this->db->all("SELECT json FROM entity_versions WHERE entity_key = 'place:test-home' ORDER BY id") as $v) {
            $doc = json_decode((string) $v['json'], true);
            self::assertIsArray($doc);
            self::assertArrayNotHasKey('chapter', $doc, 'every version, compact or spaced');
            self::assertSame(['catpope'], $doc['toons']);
        }
        self::assertSame('{"name":{"de":"P"},"x":3,"y":4,"toons":[]}', $after['place:test-plain']);
        self::assertSame('{"name":{"de":"Weg"},"toons":["catpope"],"waypoints":[]}', $after['route:test-way'], 'a route keeps its toons');
        self::assertSame('{"chapter":{"toon":"x","number":1}}', $after['chapter:test-toon:1'], 'another kind is left alone');

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->rows(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
