<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The unread relations migration (2026-11-21-unread-relations, #709): events lose `toon`, personas `introduced`,
 * chapters `featuring`, in every stored version; the rest of a document, other kinds and a second run stay as they are.
 */
final class UnreadRelationsMigrationTest extends TestCase
{
    private const KEYS = "('events', 'character:test-cat', 'character:test-plain', 'chapter:test-toon:1', 'chapter:test-toon:1:de', 'place:test-featuring')";

    private function insert(string $key, string $kind, string $json): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, '2026-11-21 00:00:00', 'draft', NULL)",
            [$key, $kind, $json]);
    }

    /** @return array<string, mixed> */
    private function doc(string $key): array
    {
        $row = $this->db->one('SELECT json FROM entity_versions WHERE entity_key = ? ORDER BY id DESC LIMIT 1', [$key]);
        $doc = json_decode((string) ($row['json'] ?? 'null'), true);
        self::assertIsArray($doc);
        return $doc;
    }

    /** @return list<string> */
    private function rows(): array
    {
        return array_map(fn(array $r) => (string) $r['json'], $this->db->all('SELECT json FROM entity_versions WHERE entity_key IN ' . self::KEYS . ' ORDER BY id'));
    }

    public function testTheUnreadFieldsLeaveEveryVersion(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $this->insert('events', 'events', '[{"title":"A","date":"2026-01-01T20:00:00","toon":["catpope"]},{"title":"B","date":"2026-02-01T20:00:00"},{"title":"C","toon":[]}]');
        $this->insert('character:test-cat', 'character', '{"name":{"de":"Kat"},"personas":[{"id":"regular","looks":[]},{"id":"biker","introduced":{"toon":"yellowmouse","season":1,"chapter":1},"looks":[{"id":"default"}]}]}');
        $this->insert('character:test-plain', 'character', '{"name":{"de":"P"},"personas":[{"id":"regular"}]}');
        $this->insert('chapter:test-toon:1', 'chapter', '{"title":{"de":"Eins"},"featuring":["catpope"],"groups":[{"label":"a"}]}');
        $this->insert('chapter:test-toon:1:de', 'chapter', '{"title": "Eins", "featuring": ["catpope", "boy"], "related_items": ["x"]}');
        $this->insert('place:test-featuring', 'place', '{"featuring":["kept"]}');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-unread-relations.mysql.sql');
        self::assertCount(3, $statements);
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->rows();

        self::assertSame([['title' => 'A', 'date' => '2026-01-01T20:00:00'], ['title' => 'B', 'date' => '2026-02-01T20:00:00'], ['title' => 'C']], $this->doc('events'), 'events in order, without toon');
        $cat = $this->doc('character:test-cat');
        self::assertSame([['id' => 'regular', 'looks' => []], ['id' => 'biker', 'looks' => [['id' => 'default']]]], $cat['personas'], 'personas in order, without introduced');
        self::assertSame(['de' => 'Kat'], $cat['name']);
        self::assertSame('{"name":{"de":"P"},"personas":[{"id":"regular"}]}', json_encode($this->doc('character:test-plain')), 'a character without introduced stays');
        self::assertSame(['title' => ['de' => 'Eins'], 'groups' => [['label' => 'a']]], $this->doc('chapter:test-toon:1'));
        self::assertSame(['title' => 'Eins', 'related_items' => ['x']], $this->doc('chapter:test-toon:1:de'), 'a per-locale chapter too');
        self::assertSame(['featuring' => ['kept']], $this->doc('place:test-featuring'), 'another kind is left alone');

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->rows(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
