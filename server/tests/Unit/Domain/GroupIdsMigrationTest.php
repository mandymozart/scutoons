<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The group ids migration (2026-11-21-group-ids, #706): a chapter group's `label` becomes its `id` in every stored
 * version; a group without one gets `g<n>` from its position (`group-<n>` when `g<n>` is taken); an existing id stays;
 * jump targets, other kinds and a second run stay as they are.
 */
final class GroupIdsMigrationTest extends TestCase
{
    private const KEYS = "('chapter:test-gid:1', 'chapter:test-gid:2', 'chapter:test-gid:3', 'chapter:test-gid:4', 'storyboard:test-gid:1')";

    private function insert(string $key, string $kind, string $json, string $status = 'draft'): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, 'test-gid', NULL, ?, NULL, '2026-11-21 00:00:00', ?, NULL)",
            [$key, $kind, $json, $status]);
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

    public function testEveryGroupGetsItsLabelAsId(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $jump = '{"type":"jumpTo","params":{"target":{"type":"chapter","value":"1","label":"forest"}}}';
        $this->insert('chapter:test-gid:1', 'chapter', '{"title":{"de":"Eins"},"groups":[{"label":"start","renderer":{"type":"vertical"},"events":[' . $jump . ']},{"label":"forest","slices":[]}]}', 'published');
        $this->insert('chapter:test-gid:2', 'chapter', '{"groups":[{"renderer":{"type":"vertical"}},{"label":"g1"},{"slices":[]},{"label":""}]}');
        $this->insert('chapter:test-gid:3', 'chapter', '{"groups":[{"id":"kept","label":"old"},{"id":"done"}]}');
        $this->insert('chapter:test-gid:4', 'chapter', '{"title":{"de":"Leer"},"groups":[]}');
        $this->insert('storyboard:test-gid:1', 'storyboard', '{"storyboard":2,"groups":[{"label":"scene-1"}]}');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-21-group-ids.mysql.sql');
        self::assertCount(2, $statements);
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->rows();

        $one = $this->doc('chapter:test-gid:1');
        self::assertSame(['de' => 'Eins'], $one['title']);
        self::assertSame(['start', 'forest'], array_column($one['groups'], 'id'));
        self::assertArrayNotHasKey('label', $one['groups'][0]);
        self::assertSame(['type' => 'chapter', 'value' => '1', 'label' => 'forest'], $one['groups'][0]['events'][0]['params']['target'], 'a jump target keeps its label');
        self::assertSame(['type' => 'vertical'], $one['groups'][0]['renderer']);
        self::assertSame(['group-1', 'g1', 'g3', 'g4'], array_column($this->doc('chapter:test-gid:2')['groups'], 'id'), 'no label: from the position; g1 is taken; an empty label is none');
        self::assertSame([['id' => 'kept'], ['id' => 'done']], $this->doc('chapter:test-gid:3')['groups'], 'an id stays, the label goes');
        self::assertSame(['title' => ['de' => 'Leer'], 'groups' => []], $this->doc('chapter:test-gid:4'), 'no groups: unchanged');
        self::assertSame([['label' => 'scene-1']], $this->doc('storyboard:test-gid:1')['groups'], 'another kind is left alone');
        $left = $this->db->one("SELECT COUNT(*) AS n FROM entity_versions WHERE entity_key IN " . self::KEYS . " AND kind = 'chapter' AND json LIKE '%\"label\":\"g1\"%'");
        self::assertSame(0, (int) ($left['n'] ?? -1));

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->rows(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
