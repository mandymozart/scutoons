<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/**
 * The scene format 2 migration (#489, 2026-11-12-scene-format-2): `scene:nave` as 1a's one-off created it (format 1, the core
 * rig flattened by hand) becomes the format-2 fixture – one holder, its space node alone – in every stored version; any other
 * format-1 scene keeps its graph and loses its `being`; a format-2 scene and the archived chapter groups stay; a second run
 * changes nothing.
 */
final class SceneFormat2MigrationTest extends TestCase
{
    private const KEYS = "('scene:nave', 'scene:other', 'scene:new', 'scene:catpope:3:intro')";

    /** @return array<string, mixed> */
    private static function fixture(string $name): array
    {
        return (array) json_decode((string) file_get_contents(self::root() . "/tests/fixtures/scenes/$name.json"), true);
    }

    /** @param array<string, mixed> $doc */
    private function insert(string $key, array $doc, string $status): void
    {
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, 'scene', NULL, NULL, ?, NULL, '2026-11-12 00:00:00', ?, NULL)",
            [$key, json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_PRESERVE_ZERO_FRACTION | JSON_THROW_ON_ERROR), $status]);
    }

    /** @return array<string, list<array<string, mixed>>> key → its versions' documents, oldest first */
    private function docs(): array
    {
        $out = [];
        foreach ($this->db->all('SELECT entity_key, json FROM entity_versions WHERE entity_key IN ' . self::KEYS . ' ORDER BY id') as $r) {
            $out[(string) $r['entity_key']][] = (array) json_decode((string) $r['json'], true);
        }
        return $out;
    }

    public function testFormatOneScenesMoveToFormatTwo(): void
    {
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
        $nave = self::fixture('nave.format-1');
        $published = ['version' => 1] + $nave;
        $draft = ['version' => 2] + $nave;
        $draft['stage']['name'] = 'The nave';
        $this->insert('scene:nave', $published, 'published');
        $this->insert('scene:nave', $draft, 'draft');
        $other = ['format' => 1, 'id' => 'other', 'version' => 1, 'toons' => [], 'stage' => ['name' => 'Other'], 'node' => [['id' => 't', 'kind' => 'tempo', 'bpm' => 120.0]], 'link' => [], 'being' => ['size' => 0.5]];
        $this->insert('scene:other', $other, 'published');
        $new = ['version' => 4] + self::fixture('templates/stage');
        $new['id'] = 'new';
        $this->insert('scene:new', $new, 'draft');
        $legacy = ['format' => 1, 'note' => 'an old chapter group under its old key'];
        $this->insert('scene:catpope:3:intro', $legacy, 'published');

        $statements = MigrationRunner::statements(self::root() . '/migrations/mysql/2026-11-12-scene-format-2.mysql.sql');
        self::assertCount(2, $statements);
        foreach ($statements as $statement) $this->db->exec($statement);
        $after = $this->docs();

        // scene:nave: the format-2 fixture, with each version's own stamp and stage
        $expected = self::fixture('nave');
        foreach ([[0, 1, 'Nave'], [1, 2, 'The nave']] as [$i, $version, $name]) {
            $doc = $after['scene:nave'][$i];
            $want = $expected;
            $want['version'] = $version;
            $want['stage']['name'] = $name;
            ksort($doc);
            ksort($want);
            self::assertEquals($want, $doc, "scene:nave v$version");
            self::assertArrayNotHasKey('being', $doc);
        }
        self::assertSame(2, $after['scene:other'][0]['format']);
        self::assertArrayNotHasKey('being', $after['scene:other'][0]);
        self::assertSame($other['node'], $after['scene:other'][0]['node'], 'another scene keeps its graph');
        self::assertEquals($new, $after['scene:new'][0], 'a format-2 scene is left alone');
        self::assertEquals($legacy, $after['scene:catpope:3:intro'][0], 'an old group key is no deck scene');

        foreach ($statements as $statement) $this->db->exec($statement);
        self::assertSame($after, $this->docs(), 'a second run changes nothing');
        $this->db->exec('DELETE FROM entity_versions WHERE entity_key IN ' . self::KEYS);
    }
}
