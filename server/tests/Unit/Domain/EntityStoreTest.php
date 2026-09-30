<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\ContentFiles;
use Scutoons\Console\ImportContentCommand;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

/** The content store by entity key (agents/DATA.md §2), the key scheme, the interchange mapping and the key migration */
final class EntityStoreTest extends TestCase
{
    public function testSavePublishRestore(): void
    {
        $store = $this->get(EntityStore::class);
        $user = $this->user();
        $key = EntityKey::parse('chapter:smoke:1:de');
        $body = (object) ['content' => (object) ['title' => 'Eins', 'params' => new \stdClass(), 'volume' => 0.5], 'base' => null, 'message' => "  first\n draft "];
        $v1 = $store->save($user, $key, $body);
        self::assertSame(['draft', 'first draft', false, 'chapter:smoke:1:de', 'chapter', 'smoke', 'de'],
            [$v1['version']['status'], $v1['version']['message'], $v1['unchanged'], $v1['version']['key'], $v1['version']['kind'], $v1['version']['toon'], $v1['version']['locale']]);
        $id1 = $v1['version']['id'];
        self::assertTrue($store->save($user, $key, (object) ['content' => $body->content])['unchanged']);
        self::assertSame('{"title":"Eins","params":{},"volume":0.5}', $store->latest('chapter:smoke:1:de')['json'] ?? null);
        try {
            $store->save($user, $key, (object) ['content' => ['x' => 1], 'base' => $id1 - 1]);
            self::fail('conflict expected');
        } catch (ApiError $error) {
            self::assertSame(['conflict', $id1], [$error->errorCode, $error->extra['latest']['id']]);
        }
        self::assertNull($store->published('chapter:smoke:1:de'));
        self::assertSame('published', $store->publish('chapter:smoke:1:de', null)['version']['status']);
        $v2 = $store->save($user, $key, (object) ['content' => ['title' => 'Zwei'], 'base' => $id1]);
        $restored = $store->restore($user, 'chapter:smoke:1:de', (object) ['id' => $id1]);
        self::assertSame("restore #$id1", $restored['version']['message']);
        self::assertSame($id1, (int) ($store->published('chapter:smoke:1:de')['id'] ?? 0), 'restore does not publish');
        try {
            $store->restore($user, 'chapter:smoke:2:de', (object) ['id' => $id1]);
            self::fail('a version of another entity');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
        $index = $store->index('smoke')['entities'];
        self::assertSame([['key' => 'chapter:smoke:1:de', 'kind' => 'chapter', 'toon' => 'smoke', 'locale' => 'de', 'latest' => $restored['version']['id'], 'published' => $id1, 'versions' => 3]], $index);
        self::assertSame([$restored['version']['id'], $v2['version']['id'], $id1], array_column($store->versions('chapter:smoke:1:de')['versions'], 'id'));
        self::assertSame(1, $store->unpublish('chapter:smoke:1:de')['unpublished']);
        $store->save($user, EntityKey::parse('characters'), (object) ['content' => ['characters' => []]]);
        self::assertSame(['characters'], array_column($store->index('')['entities'], 'key'), 'toon= empty: the platform-wide entities');
    }

    public function testKeyScheme(): void
    {
        foreach (['toon:catpope', 'toon-settings:catpope:de', 'season:catpope:1:en', 'chapter:catpope:12:de', 'scene:catpope:2:scene-14:de',
            'locations:catpope:de', 'routes:catpope:en', 'cast:catpope', 'credits:catpope', 'characters', 'moods', 'asset-requests', 'people', 'roles', 'events'] as $key) {
            self::assertSame($key, (string) EntityKey::parse($key), $key);
        }
        foreach (['chapter:catpope:1', 'chapter:catpope:0:de', 'chapter:Cat:1:de', 'toon:scutoons', 'characters:x', 'nope', 'scene:catpope:2:de', 'season:catpope:1:deu', 'items', ''] as $key) {
            self::assertFalse(EntityKey::isValid($key), $key);
        }
        $key = EntityKey::parse('scene:catpope:2:scene-14:de');
        self::assertSame(['scene', 'catpope', 2, 'scene-14', 'de', 'scene:catpope:2:scene-14', 'scene:catpope:2:scene-14:en'],
            [$key->kind, $key->toon, $key->number, $key->scene, $key->locale, $key->entity(), (string) $key->withLocale('en')]);
        self::assertSame('scene:catpope:2:scene-14', EntityKey::sceneOf('catpope', 'chapter2/scene-14.json'));
        self::assertNull(EntityKey::sceneOf('catpope', '../x.json'));
    }

    public function testInterchangeLayout(): void
    {
        $cases = [
            ['catpope', 'de', 'toon.json', 'toon-settings:catpope:de'], ['catpope', 'en', 'season1.json', 'season:catpope:1:en'],
            ['catpope', 'de', 'chapter2.json', 'chapter:catpope:2:de'], ['catpope', 'de', 'chapter2/scene-14.json', 'scene:catpope:2:scene-14:de'],
            ['catpope', 'de', 'locations.json', 'locations:catpope:de'], ['catpope', '_', 'cast.json', 'cast:catpope'],
            ['catpope', '_', 'credits.json', 'credits:catpope'], ['scutoons', '_', 'characters.json', 'characters'], ['scutoons', '_', 'people.json', 'people'],
        ];
        foreach ($cases as [$dir, $locale, $path, $key]) {
            self::assertSame($key, ContentFiles::keyOf($dir, $locale, $path), "$dir/$locale/$path");
            self::assertSame("$dir/$locale/$path", ContentFiles::pathOf(EntityKey::parse($key)), $key);
        }
        foreach ([['catpope', 'de', 'characters.json'], ['catpope', 'de', 'credits.json'], ['catpope', 'de', 'collectables.json'], ['catpope', '_', 'index.json'], ['scutoons', 'de', 'toon.json']] as [$dir, $locale, $path]) {
            self::assertNull(ContentFiles::keyOf($dir, $locale, $path), "$dir/$locale/$path is no entity");
        }
        self::assertSame('toons.json', ContentFiles::pathOf(EntityKey::parse('toon:catpope')));
        self::assertSame('events.json', ContentFiles::pathOf(EntityKey::parse('events')));
    }

    public function testSandboxStaysOutOfTheProductionSeed(): void
    {
        self::assertFalse(ImportContentCommand::includesToon('sandbox', null, true));
        self::assertTrue(ImportContentCommand::includesToon('sandbox', 'sandbox', true));
        self::assertTrue(ImportContentCommand::includesToon('sandbox', null, false));
        self::assertTrue(ImportContentCommand::includesToon('catpope', null, true));
        self::assertTrue(ImportContentCommand::includesToon(null, null, true), 'platform-wide entities');
        self::assertFalse(ImportContentCommand::includesToon('catpope', 'sandbox', false));
    }

    /** The dated migration copies every content_versions row with the same id and a key from its path */
    public function testKeyMigrationKeepsEveryVersion(): void
    {
        $file = $this->dir . '/old.sqlite';
        $db = Database::sqlite($file);
        $runner = new MigrationRunner($db, self::root() . '/migrations');
        $db->exec(implode(";\n", MigrationRunner::statements(self::root() . '/migrations/schema/qr.sqlite.sql')));
        $db->exec(implode(";\n", MigrationRunner::statements(self::root() . '/migrations/sqlite/2026-09-30-content-versions.sqlite.sql')));
        $rows = [
            ['catpope', '_', 'index.json', 'toon:catpope'], ['catpope', 'de', 'toon.json', 'toon-settings:catpope:de'],
            ['catpope', 'de', 'season1.json', 'season:catpope:1:de'], ['catpope', 'en', 'chapter12.json', 'chapter:catpope:12:en'],
            ['catpope', 'de', 'chapter2/scene-14.json', 'scene:catpope:2:scene-14:de'], ['catpope', 'de', 'locations.json', 'locations:catpope:de'],
            ['catpope', '_', 'cast.json', 'cast:catpope'], ['catpope', '_', 'credits.json', 'credits:catpope'],
            ['scutoons', '_', 'characters.json', 'characters'], ['scutoons', '_', 'asset-requests.json', 'asset-requests'], ['scutoons', '_', 'events.json', 'events'],
        ];
        foreach ($rows as $i => [$toon, $locale, $path]) {
            $db->run("INSERT INTO content_versions (id, toon, locale, path, json, created_at, status) VALUES (?, ?, ?, ?, '{}', '2026-09-30 10:00:00', ?)",
                [100 + $i, $toon, $locale, $path, $i % 2 ? 'draft' : 'published']);
        }
        foreach (MigrationRunner::statements(self::root() . '/migrations/sqlite/2026-09-30-entity-keys.sqlite.sql') as $statement) $db->exec($statement);
        foreach (MigrationRunner::statements(self::root() . '/migrations/sqlite/2026-09-30-entity-keys.sqlite.sql') as $statement) $db->exec($statement);
        $migrated = $db->all('SELECT id, entity_key, kind, toon, locale, status FROM entity_versions ORDER BY id');
        self::assertCount(count($rows), $migrated, 'idempotent');
        foreach ($rows as $i => [$toon, $locale, , $key]) {
            $parsed = EntityKey::parse($key);
            self::assertSame([100 + $i, $key, $parsed->kind, $parsed->toon, $parsed->locale, $i % 2 ? 'draft' : 'published'],
                [(int) $migrated[$i]['id'], $migrated[$i]['entity_key'], $migrated[$i]['kind'], $migrated[$i]['toon'], $migrated[$i]['locale'], $migrated[$i]['status']], $key);
        }
        unset($runner);
        $db->close();
    }
}
