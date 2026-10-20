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
        self::assertCount(1, $index);
        $core = ['key', 'kind', 'toon', 'locale', 'latest', 'published', 'versions'];
        self::assertSame(['key' => 'chapter:smoke:1:de', 'kind' => 'chapter', 'toon' => 'smoke', 'locale' => 'de', 'latest' => $restored['version']['id'], 'published' => $id1, 'versions' => 3], array_intersect_key($index[0], array_flip($core)));
        // a draft newer than the published version: its time, note and author (the creator's "publish down the tree", BACKLOG 10.60)
        self::assertArrayHasKey('latest_at', $index[0]);
        self::assertArrayHasKey('latest_message', $index[0]);
        self::assertSame($user['email'] ?? null, $index[0]['latest_author']);
        self::assertSame([$restored['version']['id'], $v2['version']['id'], $id1], array_column($store->versions('chapter:smoke:1:de')['versions'], 'id'));
        self::assertSame(1, $store->unpublish('chapter:smoke:1:de')['unpublished']);
        $store->save($user, EntityKey::parse('characters'), (object) ['content' => ['characters' => []]]);
        // the catalog documents (item:<id> …, CatalogDocs) have no toon in their key – platform-wide too
        $platform = array_column($store->index('')['entities'], 'key');
        self::assertSame(['characters'], array_values(array_filter($platform, fn(string $k) => !EntityKey::parse($k)->isCatalog())), 'toon= empty: the platform-wide entities');
        self::assertContains('item:camera', $platform);
    }

    public function testKeyScheme(): void
    {
        foreach (['toon:catpope', 'toon-settings:catpope:de', 'season:catpope:1:en', 'chapter:catpope:12:de', 'chapter:catpope:0:de',
            'locations:catpope:de', 'routes:catpope:en', 'cast:catpope', 'credits:catpope', 'script:catpope:de', 'script:yellowmouse:en', 'characters', 'moods', 'asset-requests', 'people', 'roles', 'events', 'colorations',
            'chapter:catpope:1', 'chapter:catpope:0', 'season:catpope:1',
            'season-script:catpope:1:de', 'chapter-script:toptoy:0:en', 'kanon:toon.toptoy:de', 'kanon:character.catpope:en',
            'kanon:character.catpope.human.helmet:de', 'kanon:place.cathedral:de'] as $key) {
            self::assertSame($key, (string) EntityKey::parse($key), $key);
        }
        // chapter 0 = a season's prologue; seasons start at 1; no leading zeros
        foreach (['chapter:catpope:00:de', 'chapter:catpope:01:de', 'chapter:catpope::de', 'season:catpope:0:de', 'chapter:Cat:1:de', 'toon:scutoons', 'characters:x', 'colorations:catpope', 'nope', 'scene:catpope:2', 'scene:catpope:2:scene-14', 'scene:catpope:2:scene-14:de', 'season:catpope:1:deu', 'season:catpope', 'toon-settings:catpope', 'locations:catpope', 'chapter:catpope:1:de:x', 'items', '', 'script:catpope', 'script:catpope:1:de', 'script:de',
            'season-script:catpope:0:de', 'chapter-script:catpope:01:de', 'chapter-script:catpope', 'season-script:catpope:1', 'kanon', 'kanon:place.cathedral', 'kanon:toon.scutoons:de', 'kanon:place.a.b:de',
            'kanon:character.a.b.c.d:de', 'kanon:item.x:de', 'kanon:character.catpope:de:x', 'kanon:character.catpope:deu', 'kanon:place.Cathedral:de'] as $key) {
            self::assertFalse(EntityKey::isValid($key), $key);
        }
        // one document per chapter since 2026-10-19: the scene keys are gone (their versions archived)
        $key = EntityKey::parse('chapter:catpope:2:de');
        self::assertSame(['chapter', 'catpope', 2, 'de', 'chapter:catpope:2', 'chapter:catpope:2:en'],
            [$key->kind, $key->toon, $key->number, $key->locale, $key->entity(), (string) $key->withLocale('en')]);
        self::assertSame(0, EntityKey::parse('chapter:catpope:0:de')->number);
        // chapters, seasons: the language-neutral document (no locale, agents/NEUTRAL.md) or the legacy per-locale one
        $neutral = EntityKey::parse('chapter:catpope:2');
        self::assertSame([null, true, true, 'chapter:catpope:2'], [$neutral->locale, $neutral->isNeutral(), $neutral->isNeutralDoc(), $neutral->entity()]);
        self::assertSame([false, false], [$key->isNeutral(), $key->isNeutralDoc()]);
        self::assertSame([true, false], [EntityKey::parse('toon:catpope')->isNeutral(), EntityKey::parse('toon:catpope')->isNeutralDoc()]);
        // a Kanon text (research/KANON.md): the subject is its id, a toon's text carries the toon (its toon column)
        $kanon = EntityKey::parse('kanon:character.catpope.human:de');
        self::assertSame(['kanon', null, 'character.catpope.human', 'de', 'kanon:character.catpope.human', 'kanon:character.catpope.human:en'],
            [$kanon->kind, $kanon->toon, $kanon->id, $kanon->locale, $kanon->entity(), (string) $kanon->withLocale('en')]);
        self::assertSame('toptoy', EntityKey::parse('kanon:toon.toptoy:de')->toon);
        self::assertSame([2, 'season-script:catpope:2'], [EntityKey::parse('season-script:catpope:2:de')->number, EntityKey::parse('season-script:catpope:2:de')->entity()]);
    }

    public function testInterchangeLayout(): void
    {
        $cases = [
            ['catpope', 'de', 'toon.json', 'toon-settings:catpope:de'], ['catpope', 'en', 'season1.json', 'season:catpope:1:en'],
            ['catpope', 'de', 'chapter2.json', 'chapter:catpope:2:de'], ['catpope', 'en', 'chapter0.json', 'chapter:catpope:0:en'],
            ['catpope', 'de', 'locations.json', 'locations:catpope:de'], ['catpope', '_', 'cast.json', 'cast:catpope'],
            ['catpope', '_', 'credits.json', 'credits:catpope'], ['scutoons', '_', 'characters.json', 'characters'], ['scutoons', '_', 'people.json', 'people'],
            ['scutoons', '_', 'colorations.json', 'colorations'],
            ['catpope', 'de', 'script.json', 'script:catpope:de'], ['catpope', 'en', 'script.json', 'script:catpope:en'],
            ['catpope', '_', 'chapter2.json', 'chapter:catpope:2'], ['catpope', '_', 'season1.json', 'season:catpope:1'],
            ['catpope', 'de', 'season-script1.json', 'season-script:catpope:1:de'], ['toptoy', 'en', 'chapter-script0.json', 'chapter-script:toptoy:0:en'],
            ['scutoons', 'de', 'kanon/character.catpope.human.json', 'kanon:character.catpope.human:de'], ['scutoons', 'en', 'kanon/toon.toptoy.json', 'kanon:toon.toptoy:en'],
        ];
        foreach ($cases as [$dir, $locale, $path, $key]) {
            self::assertSame($key, ContentFiles::keyOf($dir, $locale, $path), "$dir/$locale/$path");
            self::assertSame("$dir/$locale/$path", ContentFiles::pathOf(EntityKey::parse($key)), $key);
        }
        foreach ([['catpope', '_', 'script.json'], ['scutoons', 'de', 'script.json'], ['catpope', 'de', 'scripts.json'],['catpope', 'de', 'characters.json'],['catpope', 'de', 'credits.json'], ['catpope', 'de', 'collectables.json'], ['catpope', '_', 'index.json'], ['scutoons', 'de', 'toon.json'], ['scutoons', 'de', 'colorations.json'], ['catpope', '_', 'colorations.json'], ['catpope', 'de', 'season0.json'], ['catpope', 'de', 'chapter01.json'], ['catpope', '_', 'season0.json'], ['catpope', '_', 'toon.json'], ['scutoons', '_', 'chapter1.json'], ['catpope', '_', 'chapter2/scene-14.json'], ['catpope', 'de', 'chapter2/scene-14.json'], ['scutoons', '_', 'kanon/place.x.json'], ['scutoons', 'de', 'kanon/item.x.json'], ['catpope', 'de', 'season-script0.json'], ['catpope', '_', 'chapter-script1.json']] as [$dir, $locale, $path]) {
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
        self::assertSame(['script', 'storyboard', 'season-script', 'chapter-script', 'kanon'], EntityKey::CREATOR_ONLY_KINDS);
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

    /** A Kanon text and a chapter script save like the screenplay: schema-checked, per locale, a toon's text in its toon's rows */
    public function testKanonAndChapterScriptsSave(): void
    {
        $store = $this->get(EntityStore::class);
        $user = $this->user();
        $saved = $store->save($user, EntityKey::parse('kanon:toon.smoke:de'), (object) ['content' => (object) ['text' => "## Prämisse
Eine Katze.
"]]);
        self::assertSame(['kanon', 'smoke', 'de'], [$saved['version']['kind'], $saved['version']['toon'], $saved['version']['locale']]);
        $saved = $store->save($user, EntityKey::parse('kanon:character.catpope.human:en'), (object) ['content' => (object) ['text' => "## Look
Pale.
"]]);
        self::assertSame(['kanon', null, 'en'], [$saved['version']['kind'], $saved['version']['toon'], $saved['version']['locale']]);
        $saved = $store->save($user, EntityKey::parse('chapter-script:smoke:1:de'), (object) ['content' => (object) ['text' => "1   INT. KÜCHE - NACHT
"]]);
        self::assertSame(['chapter-script', 'smoke', 'de'], [$saved['version']['kind'], $saved['version']['toon'], $saved['version']['locale']]);
        // the API's write path checks the type (ContentSchemas): a Kanon text is { text: string }
        $schemas = $this->get(\Scutoons\Domain\Content\ContentSchemas::class);
        self::assertSame([], $schemas->problemsOf(EntityKey::parse('kanon:place.cathedral:de'), (object) ['text' => 'x']));
        self::assertNotSame([], $schemas->problemsOf(EntityKey::parse('kanon:place.cathedral:de'), (object) ['text' => 3]));
        self::assertNotSame([], $schemas->problemsOf(EntityKey::parse('season-script:smoke:1:de'), (object) ['meta' => (object) []]));
    }

    /** 2026-10-06-show-text-box: the event type `showOnomatopeia` → `showTextBox` in every stored version, texts untouched */
    public function testTheShowTextBoxMigration(): void
    {
        $json = '{"groups":[{"slices":[{"events":[{"type":"showOnomatopeia","params":{"dialog":"b1"}}],"dialog":[{"id":"b1","text":{"de":"kein showOnomatopeia hier"}}]}]}]}';
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('chapter:smoke:1', 'chapter', 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-show-text-box.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-show-text-box.mysql.sql'); // idempotent
        self::assertSame(str_replace('"showOnomatopeia"', '"showTextBox"', $json), $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:1'"));
        self::assertStringContainsString('kein showOnomatopeia hier', (string) $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:1'"));
    }

    /** 2026-10-06-clock-syntax: `{name:clock}` → `{clock(name)}` in every stored version; other braces untouched */
    public function testTheClockSyntaxMigration(): void
    {
        $json = '{"groups":[{"events":[{"type":"showMessage","params":{"text":{"de":"Es ist {time:clock} Uhr, {who}.","en":"{_t2:clock}{x:clock2} {a b:clock} {time: clock}"}}}]}]}';
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('chapter:smoke:1', 'chapter', 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-clock-syntax.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-clock-syntax.mysql.sql'); // idempotent
        self::assertSame(
            '{"groups":[{"events":[{"type":"showMessage","params":{"text":{"de":"Es ist {clock(time)} Uhr, {who}.","en":"{clock(_t2)}{x:clock2} {a b:clock} {time: clock}"}}}]}]}',
            $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:1'")
        );
    }

    /**
     * 2026-10-12-map-documents (agents/MAP-DOCUMENTS.md, owner D4 "only newest"): per place / route the version of the
     * published map (published) + the newest map's where it differs or is new (draft) + a removal draft where the newest
     * dropped a published one; older versions are not split; the lists leave every map version; idempotent
     */
    public function testTheMapDocumentsMigration(): void
    {
        $place = fn(string $id, int $x) => ['id' => $id, 'name' => ['de' => strtoupper($id)], 'x' => $x, 'y' => 1, 'toons' => ['t']];
        $route = fn(array $ids) => ['id' => 'r', 'name' => ['de' => 'R'], 'toons' => ['t'], 'waypoints' => array_map(fn($i) => ['location' => $i], $ids)];
        $versions = [
            1 => ['draft', [$place('a', 1), $place('old', 7)], [$route(['a', 'old'])]],
            2 => ['published', [$place('a', 5), $place('b', 2), $place('c', 3)], [$route(['a', 'b'])]],
            3 => ['draft', [$place('a', 6), $place('b', 2)], [$route(['a', 'b'])]],
            4 => ['draft', [$place('a', 5), $place('c', 4), $place('d', 9)], [$route(['a', 'c'])]],
        ];
        $mapIds = [];
        foreach ($versions as $n => [$status, $locations, $routes]) {
            $json = json_encode(['width' => 10, 'height' => 10, 'layers' => [['id' => 'terrain', 'image' => 'x.svg']], 'locations' => $locations, 'routes' => $routes]);
            $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('map', 'map', NULL, NULL, ?, 'u1', ?, ?, ?)",
                [$json, "2026-10-0$n 00:00:00", $status, $n === 4 ? 'b goes' : null]);
            $mapIds[$n] = $this->db->lastInsertId();
        }
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-12-map-documents.mysql.sql');
        $rows = fn() => $this->db->all("SELECT entity_key, json, status, author, created_at, message FROM entity_versions WHERE kind IN ('place', 'route') ORDER BY id");
        $first = $rows();
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-12-map-documents.mysql.sql');
        self::assertSame($first, $rows(), 'a second run changes nothing');

        $line = fn(array $r) => [$r['entity_key'], $r['status'], json_decode((string) $r['json'], true)];
        $p = fn(string $id, int $x, bool $removed = false) => ['name' => ['de' => strtoupper($id)], 'x' => $x, 'y' => 1, 'toons' => ['t']] + ($removed ? ['removed' => true] : []);
        $r = fn(array $ids) => ['name' => ['de' => 'R'], 'toons' => ['t'], 'waypoints' => array_map(fn($i) => ['location' => $i], $ids)];
        self::assertSame([
            ['place:a', 'published', $p('a', 5)],
            ['place:b', 'published', $p('b', 2)],
            ['place:c', 'published', $p('c', 3)],
            ['route:r', 'published', $r(['a', 'b'])],
            ['place:c', 'draft', $p('c', 4)],
            ['place:d', 'draft', $p('d', 9)],
            ['route:r', 'draft', $r(['a', 'c'])],
            ['place:b', 'draft', $p('b', 2, true)],
        ], array_map($line, $first), 'published map first, then the newest draft where it differs; "old" (only in an older draft) and v3 of a are not carried over');
        self::assertSame(['u1', '2026-10-04 00:00:00', 'map v' . ($mapIds[4] ?? 0) . ': b goes'], [$first[4]['author'], $first[4]['created_at'], $first[4]['message']]);

        // the map versions keep only their settings – same ids, same status
        foreach ($this->db->all("SELECT id, json, status FROM entity_versions WHERE entity_key = 'map' ORDER BY id") as $i => $m) {
            self::assertSame(['width' => 10, 'height' => 10, 'layers' => [['id' => 'terrain', 'image' => 'x.svg']]], json_decode((string) $m['json'], true));
            self::assertSame($i === 1 ? 'published' : 'draft', $m['status']);
        }
    }

    /**
     * 2026-10-12-character-documents (owner D10): the registry split like the map – per character the published
     * registry's entry + the newest's where it differs or is new + a removal draft; the list leaves every version
     */
    public function testTheCharacterDocumentsMigration(): void
    {
        $c = fn(string $id, string $color) => ['id' => $id, 'name' => ['de' => strtoupper($id)], 'color' => $color, 'personas' => []];
        $versions = [['published', [$c('cat', 'blue'), $c('boy', 'red')]], ['draft', [$c('cat', 'green'), $c('frog', 'lime')]]];
        foreach ($versions as $i => [$status, $characters]) {
            $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, 'u1', ?, ?, NULL)",
                [json_encode(['characters' => $characters]), '2026-10-0' . ($i + 1) . ' 00:00:00', $status]);
        }
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-12-character-documents.mysql.sql');
        $rows = fn() => $this->db->all("SELECT entity_key, json, status FROM entity_versions WHERE kind = 'character' ORDER BY id");
        $first = $rows();
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-12-character-documents.mysql.sql');
        self::assertSame($first, $rows(), 'a second run changes nothing');
        $doc = fn(string $id, string $color, bool $removed = false) => ['name' => ['de' => strtoupper($id)], 'color' => $color, 'personas' => []] + ($removed ? ['removed' => true] : []);
        self::assertSame([
            ['character:cat', 'published', $doc('cat', 'blue')],
            ['character:boy', 'published', $doc('boy', 'red')],
            ['character:cat', 'draft', $doc('cat', 'green')],
            ['character:frog', 'draft', $doc('frog', 'lime')],
            ['character:boy', 'draft', $doc('boy', 'red', true)],
        ], array_map(fn(array $r) => [$r['entity_key'], $r['status'], json_decode((string) $r['json'], true)], $first));
        self::assertSame(['{}', '{}'], array_column($this->db->all("SELECT json FROM entity_versions WHERE entity_key = 'characters' ORDER BY id"), 'json'));
    }

    /**
     * 2026-10-06-target-value: a jump target's numeric value becomes a string (chapter / season / toon / product / xr);
     * strings, other objects with a numeric value and texts stay; a second run changes nothing
     */
    public function testTheTargetValueMigration(): void
    {
        $json = '{"events":[{"type":"addChoice","params":{"label":{"de":"Weiter \\"type\\":\\"chapter\\",\\"value\\":7"},"onSelect":{"jumpTo":{"target":{"type":"chapter","value":2,"label":"scene-14"}}}}},'
            . '{"type":"jumpTo","params":{"target":{"type":"season","value":12}}},{"type":"jumpTo","params":{"target":{"type":"chapter","value":"3"}}},'
            . '{"type":"setVariable","params":{"variable":"time","value":15}},{"type":"slider","value":4}]}';
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('scene:smoke:2:scene-14', 'scene', 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-target-value.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-target-value.mysql.sql'); // idempotent
        self::assertSame(
            '{"events":[{"type":"addChoice","params":{"label":{"de":"Weiter \\"type\\":\\"chapter\\",\\"value\\":7"},"onSelect":{"jumpTo":{"target":{"type":"chapter","value":"2","label":"scene-14"}}}}},'
            . '{"type":"jumpTo","params":{"target":{"type":"season","value":"12"}}},{"type":"jumpTo","params":{"target":{"type":"chapter","value":"3"}}},'
            . '{"type":"setVariable","params":{"variable":"time","value":15}},{"type":"slider","value":4}]}',
            $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'scene:smoke:2:scene-14'")
        );
    }

    /**
     * 2026-10-06-repeat: `once` (triggers, placed items) → `repeat` (false → every time, written out); a VN trigger /
     * tap-event item without once (it meant true) gets once per page view; audio sample triggers, texts and other
     * objects stay as they are; a second run changes nothing
     */
    public function testTheRepeatMigration(): void
    {
        $slice = '{"image":"a.webp","audio":{"audio":{"samples":[{"id":"bell","trigger":{"type":"click","mode":"poly","rect":{"x":1,"y":2,"width":3,"height":4}},"volume":1,"repeat":2}]}},'
            . '"events":['
            . '{"type":"showMessage","params":{"text":{"de":"Ein \\"trigger\\":{ und {once} – \\"once\\":true bleibt"},"wait":true},"trigger":{"type":"click","rect":{"x":1,"y":2,"width":3,"height":4}}},'
            . '{"type":"showChoices","trigger":{"type":"intersect"}},'
            . '{"trigger":{"once":false,"type":"hover"},"type":"hideMessage"},'
            . '{"type":"showTextBox","params":{"dialog":"b1"},"trigger":{"type":"intersect","once":true}},'
            . '{"type":"showTextBox","params":{"dialog":"b2"},"trigger":{"type":"click","mode":"poly","rect":{"x":1,"y":1,"width":1,"height":1}}},'
            . '{"type":"showTextBox","params":{"dialog":"b3"},"trigger":{"type":"click","repeat":{"mode":"once"}}}'
            . '],"items":['
            . '{"id":"coin-1","item":"coin","rect":{"x":1,"y":1,"width":5},"action":"events","events":[{"type":"showMessage","params":{"text":{"de":"{a|b}"}}}]},'
            . '{"id":"coin-2","item":"coin","rect":{"x":1,"y":1,"width":5},"action":"events","events":[{"type":"hideMessage"}],"once":false},'
            . '{"id":"coin-3","item":"coin","action":"events","once":true,"events":[]},'
            . '{"id":"coin-4","item":"coin","action":"pickup","rect":{"x":1,"y":1,"width":5}}'
            . ']}';
        $json = '{"groups":[{"label":"g","renderer":{"type":"vertical"},"slices":[' . $slice . '],"events":[{"type":"showMessage","params":{"text":{"de":"x"}}}]}]}';
        $view = '"repeat":{"mode":"view"}';
        $expected = '{"groups":[{"label":"g","renderer":{"type":"vertical"},"slices":[{"image":"a.webp","audio":{"audio":{"samples":[{"id":"bell","trigger":{"type":"click","mode":"poly","rect":{"x":1,"y":2,"width":3,"height":4}},"volume":1,"repeat":2}]}},'
            . '"events":['
            . '{"type":"showMessage","params":{"text":{"de":"Ein \\"trigger\\":{ und {once} – \\"once\\":true bleibt"},"wait":true},"trigger":{"type":"click","rect":{"x":1,"y":2,"width":3,"height":4},' . $view . '}},'
            . '{"type":"showChoices","trigger":{"type":"intersect",' . $view . '}},'
            . '{"trigger":{"repeat":{"mode":"every"},"type":"hover"},"type":"hideMessage"},'
            . '{"type":"showTextBox","params":{"dialog":"b1"},"trigger":{"type":"intersect",' . $view . '}},'
            . '{"type":"showTextBox","params":{"dialog":"b2"},"trigger":{"type":"click","mode":"poly","rect":{"x":1,"y":1,"width":1,"height":1},' . $view . '}},'
            . '{"type":"showTextBox","params":{"dialog":"b3"},"trigger":{"type":"click","repeat":{"mode":"once"}}}'
            . '],"items":['
            . '{"id":"coin-1","item":"coin","rect":{"x":1,"y":1,"width":5},"action":"events","events":[{"type":"showMessage","params":{"text":{"de":"{a|b}"}}}],' . $view . '},'
            . '{"id":"coin-2","item":"coin","rect":{"x":1,"y":1,"width":5},"action":"events","events":[{"type":"hideMessage"}],"repeat":{"mode":"every"}},'
            . '{"id":"coin-3","item":"coin","action":"events",' . $view . ',"events":[]},'
            . '{"id":"coin-4","item":"coin","action":"pickup","rect":{"x":1,"y":1,"width":5}}'
            . ']}],"events":[{"type":"showMessage","params":{"text":{"de":"x"}}}]}]}';
        self::assertNotNull(json_decode($json), 'the input is JSON');
        self::assertNotNull(json_decode($expected), 'the expectation is JSON');
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('chapter:smoke:1', 'chapter', 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $stored = fn(): mixed => $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:1'");
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-repeat.mysql.sql');
        $first = $stored();
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-06-repeat.mysql.sql'); // idempotent
        self::assertSame([$expected, $expected], [$first, $stored()]);
    }

    /**
     * 2026-10-07-give-item: `add` of an item-named story variable (a catalog id, with or without a domain) becomes a give –
     * a choice's action and a setVariable event; other variables, a `set`, a qty over 10 and texts stay; idempotent
     */
    public function testTheGiveItemMigration(): void
    {
        foreach (['cucumber', 'repair_coupon'] as $i => $id) {
            $this->db->run("INSERT IGNORE INTO catalog_items (id, position, toon, edition_id, data, updated_at, updated_by) VALUES (?, ?, NULL, NULL, '{}', '2026-10-01 00:00:00', NULL)", [$id, $i]);
        }
        $json = '{"groups":[{"events":['
            . '{"type":"addChoice","params":{"label":{"de":"\\"add\\":{\\"variable\\":\\"cucumber\\",\\"value\\":1}}"},"onSelect":{"add":{"variable":"cucumber","value":1}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"b"},"onSelect":[{"add":{"variable":"catpope.repair_coupon","value":2}},{"add":{"variable":"minutes","value":15}},{"jumpTo":{"target":{"type":"chapter","value":"2"}}}]}},'
            . '{"type":"addChoice","params":{"label":{"de":"c"},"onSelect":{"set":{"variable":"cucumber","value":"1"}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"d"},"onSelect":{"add":{"variable":"cucumber","value":11}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"e"},"onSelect":{"add":{"variable":"cucumbers","value":1}}}},'
            . '{"type":"setVariable","params":{"variable":"global.cucumber","value":3,"op":"add"}},'
            . '{"type":"setVariable","params":{"variable":"cucumber","value":1,"op":"set"}}'
            . ']}]}';
        $expected = '{"groups":[{"events":['
            . '{"type":"addChoice","params":{"label":{"de":"\\"add\\":{\\"variable\\":\\"cucumber\\",\\"value\\":1}}"},"onSelect":{"give":{"item":"cucumber","qty":1}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"b"},"onSelect":[{"give":{"item":"repair_coupon","qty":2}},{"add":{"variable":"minutes","value":15}},{"jumpTo":{"target":{"type":"chapter","value":"2"}}}]}},'
            . '{"type":"addChoice","params":{"label":{"de":"c"},"onSelect":{"set":{"variable":"cucumber","value":"1"}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"d"},"onSelect":{"add":{"variable":"cucumber","value":11}}}},'
            . '{"type":"addChoice","params":{"label":{"de":"e"},"onSelect":{"add":{"variable":"cucumbers","value":1}}}},'
            . '{"type":"giveItem","params":{"item":"cucumber","qty":3}},'
            . '{"type":"setVariable","params":{"variable":"cucumber","value":1,"op":"set"}}'
            . ']}]}';
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('scene:smoke:1:s', 'scene', 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-07-give-item.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-07-give-item.mysql.sql'); // idempotent
        self::assertSame($expected, $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'scene:smoke:1:s'"));
    }

    /**
     * 2026-10-16-endless-strip (FEATURES 10.100): a horizontal group's params keep only `endless` – fullWidth → none (even
     * with a loop), loop → endless, anything else → none, params on a vertical group go; a video's / sample's loop stays;
     * every group of a chapter (eight with old keys: one pass), a scene's renderer; a second run changes 0 rows
     */
    public function testTheEndlessStripMigration(): void
    {
        $slice = '{"image":"a.webp","video":{"src":"v.mp4","loop":true},"audio":{"audio":{"samples":[{"id":"bell","loop":true}]}}}';
        $group = fn(string $renderer) => '{"label":"g","renderer":' . $renderer . ',"slices":[' . $slice . ']}';
        $before = [
            '{"type":"horizontal","params":{"loop":true,"autoPlay":{"enabled":false},"fullWidth":true}}',
            '{"type":"vertical"}',
            '{"type":"horizontal","params":{"loop":true,"freeMode":true}}',
            '{"type":"horizontal","params":{"freeMode":true,"keyboard":true}}',
            '{"type":"vertical","params":{"loop":true,"cube":true}}',
            '{"type":"horizontal","params":{"endless":true}}',
            '{"type":"horizontal"}',
        ];
        $after = [
            '{"type":"horizontal"}',
            '{"type":"vertical"}',
            '{"type":"horizontal","params":{"endless":true}}',
            '{"type":"horizontal"}',
            '{"type":"vertical"}',
            '{"type":"horizontal","params":{"endless":true}}',
            '{"type":"horizontal"}',
        ];
        // twice: eight groups with old keys in one chapter version, all fixed in one pass
        $chapter = '{"groups":[' . implode(',', array_map($group, [...$before, ...$before])) . ']}';
        $expected = '{"groups":[' . implode(',', array_map($group, [...$after, ...$after])) . ']}';
        $scenes = [
            'scene:smoke:1:swiper' => [$group('{"type":"horizontal","params":{"freeMode":true,"loop":true}}'), $group('{"type":"horizontal","params":{"endless":true}}')],
            'scene:smoke:1:strip' => [$group('{"type":"horizontal","params":{"fullWidth":true,"loop":true}}'), $group('{"type":"horizontal"}')],
            'scene:smoke:1:plain' => [$group('{"type":"horizontal","params":{"flip":true}}'), $group('{"type":"horizontal"}')],
            'scene:smoke:1:column' => [$group('{"type":"vertical","params":{"loop":true}}'), $group('{"type":"vertical"}')],
            'scene:smoke:1:done' => [$group('{"type":"horizontal","params":{"endless":true}}'), $group('{"type":"horizontal","params":{"endless":true}}')],
        ];
        $insert = "INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', ?, NULL)";
        foreach (['published', 'draft'] as $status) $this->db->run($insert, ['chapter:smoke:1', 'chapter', $chapter, $status]);
        foreach ($scenes as $key => [$json]) $this->db->run($insert, [$key, 'scene', $json, 'draft']);
        foreach ([$chapter, $expected, ...array_merge(...array_values($scenes))] as $json) self::assertNotNull(json_decode($json), 'JSON');

        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $apply = fn(): int => $runner->applyFile('2026-10-16-endless-strip.mysql.sql', fn(string $line) => null)['rows'];
        self::assertSame(2 + 4, $apply(), 'each chapter version once (all its groups in one pass), four scenes');
        self::assertSame([$expected, $expected], array_column($this->db->all("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:1' ORDER BY id"), 'json'));
        foreach ($scenes as $key => [, $after]) self::assertSame($after, $this->db->value('SELECT json FROM entity_versions WHERE entity_key = ?', [$key]), $key);
        self::assertSame(0, $apply(), 'a second run changes nothing');
    }

    /**
     * 2026-10-19-inline-scenes (one document per chapter, research/TEXT-ACROSS-SLICES.md §5 step 3): a `ref` group becomes
     * the scene's group – the current published chapter gets the newest published scene, the newest chapter the newest
     * scene, an older one the scene as it was then (a published one: the newest published then); a missing scene keeps its
     * group; the scene versions move to entity_versions_legacy with their asset usage rows; a second run changes 0 rows
     */
    public function testTheInlineScenesMigration(): void
    {
        $insert = "INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, 'smoke', NULL, ?, NULL, '2026-10-01 00:00:00', ?, NULL)";
        $chapter = '{"title":{"de":"Zwei"},"groups":[{"label":"own","slices":[]},{"ref":{"src":"chapter2/a.json"}},{"ref":{"src":"chapter2/gone.json"}}]}';
        $scene = fn(string $label) => '{"label":"' . $label . '","renderer":{"type":"vertical"},"slices":[{"image":"' . $label . '.webp","width":390,"height":625}]}';
        /** @var array<string, int> $ids */
        $ids = [];
        foreach ([['scene', 'a1', 'published'], ['chapter', 'v1', 'draft'], ['scene', 'a2', 'draft'], ['chapter', 'v2', 'published'], ['scene', 'a3', 'published'],
            ['chapter', 'v4', 'published'], ['scene', 'a4', 'draft'], ['chapter', 'v5', 'draft']] as [$kind, $name, $status]) {
            $this->db->run($insert, $kind === 'scene' ? ['scene:smoke:2:a', 'scene', $scene($name), $status] : ['chapter:smoke:2', 'chapter', $chapter, $status]);
            $ids[$name] = (int) $this->db->value('SELECT MAX(id) FROM entity_versions');
        }
        $this->db->run($insert, ['chapter:smoke:3', 'chapter', '{"groups":[{"label":"plain","slices":[]}]}', 'published']);
        $this->db->run("INSERT INTO asset_usages (path, source, ref, field, state, version_id, toon, raw, scanned_at) VALUES ('smoke/a4.webp', 'content', 'scene:smoke:2:a', '/slices/0/image', 'draft', ?, 'smoke', 'a4.webp', '2026-10-01 00:00:00')", [$ids['a4']]);

        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $apply = fn(): int => $runner->applyFile('2026-10-19-inline-scenes.mysql.sql', fn(string $line) => null)['rows'];
        self::assertSame(4 + 4 + 1 + 4, $apply(), 'four chapter versions, four scene versions copied, one usage row, four scene versions deleted');
        $groups = fn(string $version) => json_decode((string) $this->db->value('SELECT json FROM entity_versions WHERE id = ?', [$ids[$version]]), true)['groups'];
        foreach (['v1' => 'a1', 'v2' => 'a1', 'v4' => 'a3', 'v5' => 'a4'] as $version => $label) {
            self::assertSame([['label' => 'own', 'slices' => []], json_decode($scene($label), true), ['ref' => ['src' => 'chapter2/gone.json']]], $groups($version), "$version gets $label");
        }
        self::assertSame('{"groups":[{"label":"plain","slices":[]}]}', $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'chapter:smoke:3'"), 'no ref: untouched');
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE kind = 'scene'"));
        self::assertSame([$ids['a1'], $ids['a2'], $ids['a3'], $ids['a4']], array_map('intval', array_column($this->db->all("SELECT id FROM entity_versions_legacy WHERE entity_key = 'scene:smoke:2:a' ORDER BY id"), 'id')), 'kept as history');
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM asset_usages WHERE ref = 'scene:smoke:2:a'"));
        self::assertSame(0, $apply(), 'a second run changes nothing (the group of the missing scene is matched again and stays as it is)');
    }
}
