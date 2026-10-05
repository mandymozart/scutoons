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
        foreach (['toon:catpope', 'toon-settings:catpope:de', 'season:catpope:1:en', 'chapter:catpope:12:de', 'chapter:catpope:0:de', 'scene:catpope:2:scene-14:de', 'scene:catpope:0:intro:en',
            'locations:catpope:de', 'routes:catpope:en', 'cast:catpope', 'credits:catpope', 'script:catpope:de', 'script:yellowmouse:en', 'characters', 'moods', 'asset-requests', 'people', 'roles', 'events'] as $key) {
            self::assertSame($key, (string) EntityKey::parse($key), $key);
        }
        // chapter 0 = a season's prologue; seasons start at 1; no leading zeros
        foreach (['chapter:catpope:1', 'chapter:catpope:00:de', 'chapter:catpope:01:de', 'chapter:catpope::de', 'season:catpope:0:de', 'chapter:Cat:1:de', 'toon:scutoons', 'characters:x', 'nope', 'scene:catpope:2:de', 'season:catpope:1:deu', 'items', '', 'script:catpope', 'script:catpope:1:de', 'script:de'] as $key) {
            self::assertFalse(EntityKey::isValid($key), $key);
        }
        $key = EntityKey::parse('scene:catpope:2:scene-14:de');
        self::assertSame(['scene', 'catpope', 2, 'scene-14', 'de', 'scene:catpope:2:scene-14', 'scene:catpope:2:scene-14:en'],
            [$key->kind, $key->toon, $key->number, $key->scene, $key->locale, $key->entity(), (string) $key->withLocale('en')]);
        self::assertSame('scene:catpope:2:scene-14', EntityKey::sceneOf('catpope', 'chapter2/scene-14.json'));
        self::assertNull(EntityKey::sceneOf('catpope', '../x.json'));
        self::assertSame('scene:catpope:0:intro', EntityKey::sceneOf('catpope', 'chapter0/intro.json'));
        self::assertSame(0, EntityKey::parse('chapter:catpope:0:de')->number);
    }

    public function testInterchangeLayout(): void
    {
        $cases = [
            ['catpope', 'de', 'toon.json', 'toon-settings:catpope:de'], ['catpope', 'en', 'season1.json', 'season:catpope:1:en'],
            ['catpope', 'de', 'chapter2.json', 'chapter:catpope:2:de'], ['catpope', 'en', 'chapter0.json', 'chapter:catpope:0:en'], ['catpope', 'de', 'chapter2/scene-14.json', 'scene:catpope:2:scene-14:de'],
            ['catpope', 'de', 'locations.json', 'locations:catpope:de'], ['catpope', '_', 'cast.json', 'cast:catpope'],
            ['catpope', '_', 'credits.json', 'credits:catpope'], ['scutoons', '_', 'characters.json', 'characters'], ['scutoons', '_', 'people.json', 'people'],
            ['catpope', 'de', 'script.json', 'script:catpope:de'], ['catpope', 'en', 'script.json', 'script:catpope:en'],
        ];
        foreach ($cases as [$dir, $locale, $path, $key]) {
            self::assertSame($key, ContentFiles::keyOf($dir, $locale, $path), "$dir/$locale/$path");
            self::assertSame("$dir/$locale/$path", ContentFiles::pathOf(EntityKey::parse($key)), $key);
        }
        foreach ([['catpope', '_', 'script.json'], ['scutoons', 'de', 'script.json'], ['catpope', 'de', 'scripts.json'],['catpope', 'de', 'characters.json'],['catpope', 'de', 'credits.json'], ['catpope', 'de', 'collectables.json'], ['catpope', '_', 'index.json'], ['scutoons', 'de', 'toon.json'], ['catpope', 'de', 'season0.json'], ['catpope', 'de', 'chapter01.json']] as [$dir, $locale, $path]) {
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

    /** Screenplays are creator material: out of the production seed unless named, imported locally like the rest */
    public function testScriptsStayOutOfTheProductionSeed(): void
    {
        self::assertSame(['script'], EntityKey::CREATOR_ONLY_KINDS);
        self::assertFalse(ImportContentCommand::includesKind('script', [], true));
        self::assertTrue(ImportContentCommand::includesKind('script', ['script'], true));
        self::assertTrue(ImportContentCommand::includesKind('script', [], false));
        self::assertTrue(ImportContentCommand::includesKind('chapter', [], true));
    }

    /** A screenplay keeps its text exactly: indentation, trailing spaces, blank lines, tabs, umlauts */
    public function testScriptTextIsStoredExactly(): void
    {
        $store = $this->get(EntityStore::class);
        $user = $this->user();
        $text = "    # 1. Kapitel\n\n    # DENIAL\n\n2   INT. SCHLAFZIMMER - NACHT\n\n                            ERZÄHLER:IN (V.O.)\n                EIN NEUER TAG!  \n\tTab\n\n\n";
        $key = EntityKey::parse('script:smoke:de');
        $saved = $store->save($user, $key, (object) ['content' => (object) ['text' => $text, 'meta' => (object) ['title' => 'Katzenpabst']]]);
        self::assertSame(['script', 'smoke', 'de'], [$saved['version']['kind'], $saved['version']['toon'], $saved['version']['locale']]);
        $store->publish('script:smoke:de', null);
        $read = json_decode((string) $store->read('script:smoke:de', false, 0)['json']);
        self::assertSame($text, $read->text);
    }

}
