<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarSlots;
use Scutoons\Domain\Catalog\Catalog;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogSeed;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Catalog\CatalogValidator;
use Scutoons\Domain\Catalog\Editions;
use Scutoons\Domain\Catalog\FeaturedProducts;
use Scutoons\Domain\Catalog\ItemRespawn;
use Scutoons\Domain\Catalog\Products;
use Scutoons\Domain\Catalog\Toons;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Library\Library;
use Scutoons\Domain\Reader\ReaderStats;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Support\Json;
use Scutoons\Tests\TestCase;

/**
 * The catalog in the database (agents/CATALOG.md): the migrated tables serve the fixture (server/tests/fixtures/catalog
 * = the frozen 2026-10-04 snapshot of the migration), seeding from catalog files, the code constants (slots, stats)
 * against @scutoons/content, validation, history + undo, toon creation, the folds of the retired settings.
 */
final class CatalogTest extends TestCase
{
    private const BY = 'admin:boss@example.com';

    private static function config(string $file): string
    {
        return (string) file_get_contents(self::root() . '/' . CatalogSeed::FIXTURE . "/$file");
    }

    private static function fixture(): CatalogSeed
    {
        return new CatalogSeed(self::root() . '/' . CatalogSeed::FIXTURE);
    }

    /** JSON with objects kept, slashes / unicode unescaped, 1.0 kept – compares documents, not their whitespace */
    private static function canonical(string $json): string
    {
        return (string) json_encode(json_decode($json), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
    }

    /**
     * What GET /api/items/ answers for the fixture's items.json: the file (+ slots / stats, which are code) decoded as
     * arrays, encoded with the API's flags
     */
    private static function servedItemsBefore(): string
    {
        $file = json_decode(self::config('items.json'), true);
        $before = ['version' => $file['version'], 'slots' => AvatarSlots::ALL, 'items' => $file['items'], 'recipes' => $file['recipes'], 'albums' => $file['albums'], 'stats' => ReaderStats::DEFINITIONS, 'home_scenes' => $file['home_scenes'], 'airlift_carriers' => $file['airlift_carriers']];
        return Json::encode($before);
    }

    private function assertServesTheSeedFiles(string $when): void
    {
        self::assertSame(self::servedItemsBefore(), Json::encode($this->get(Catalog::class)->data()), "items ($when)");
        $products = $this->get(Products::class);
        self::assertSame(self::canonical(self::config('products.json')), self::canonical($products->catalogJson()), "products ($when)");
        self::assertSame(self::canonical(self::config('featured-products.json')), self::canonical($this->get(FeaturedProducts::class)->json()), "featured ($when)");
        self::assertSame(json_decode(self::config('editions.json'), true), $this->get(Editions::class)->all(), "editions ($when)");
        self::assertSame(json_decode(self::config('toons.json'), true), $this->get(Toons::class)->all(), "toons ($when)");
        self::assertSame(json_decode(self::config('permissions.json'), true), $this->get(Permissions::class)->map(), "permissions ($when)");
    }

    public function testTheMigratedTablesServeWhatTheFilesServed(): void
    {
        $this->assertServesTheSeedFiles('migration data');
        self::assertSame(27, (int) $this->db->value('SELECT COUNT(*) FROM catalog_items'));
        self::assertSame(['1_catpope_dago', '2_catpope_beetles', '3_catpope_vinyl'], array_column($this->db->all('SELECT id FROM catalog_editions ORDER BY position'), 'id'));
        self::assertSame(['dago_hoodie', '1_catpope_dago'], array_values($this->db->one("SELECT item_id, edition_id FROM catalog_products WHERE id = 'dago_hoodie'") ?? []), 'real relation columns');
        self::assertSame(['dago_hoodie', 'beige'], array_values($this->db->one("SELECT product_id, variant_id FROM catalog_featured WHERE id = 'dago_hoodie:beige'") ?? []));
        // seeding from the files replaces the rows: same answers
        $counts = self::fixture()->seed($this->db, true);
        self::assertSame(27, $counts['items']);
        $this->get(CatalogStore::class)->forget();
        $this->assertServesTheSeedFiles('seeded from the fixture');
        // the airlift carriers start empty (the owner adds them – research/CAVE.md D8): nothing to leave alone
        self::assertSame(array_merge(array_fill_keys(array_keys(CatalogStore::KINDS), null), ['airlift-carriers' => 0]), self::fixture()->seed($this->db), 'filled tables are left alone');
    }

    public function testTheMigrationFoldsTheRetiredSettings(): void
    {
        // a database from before 2026-10-04: admin overrides in settings, empty catalog tables
        foreach (CatalogStore::KINDS as $def) $this->db->exec('DELETE FROM ' . $def['table']);
        $this->db->run('INSERT INTO settings (`key`, `value`, updated_at, updated_by) VALUES (?, ?, ?, ?)', [ItemRespawn::SETTING, '{"cucumber":{"default":"visit","min":"visit","max":{"days":1}},"tuna_can":null}', '2026-10-02 10:00:00', 'admin:old@example.com']);
        $this->db->run('INSERT INTO settings (`key`, `value`, updated_at, updated_by) VALUES (?, ?, ?, ?)', [FeaturedProducts::SETTING, '[{"product":"scorpion_amulet"},{"product":"dago_hoodie","variant":"black"}]', '2026-10-03 10:00:00', 'admin:old@example.com']);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-04-catalog.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-04-catalog.mysql.sql'); // idempotent
        $store = $this->get(CatalogStore::class);
        $store->forget();
        self::assertSame(['default' => 'visit', 'min' => 'visit', 'max' => ['days' => 1]], $store->get('items', 'cucumber')['respawn'] ?? null);
        $tuna = $store->get('items', 'tuna_can');
        self::assertTrue(is_array($tuna) && array_key_exists('respawn', $tuna) && $tuna['respawn'] === null, 'an override null = never reappears');
        self::assertSame('admin:old@example.com', $store->row('items', 'cucumber')['updated_by'] ?? null);
        self::assertSame(self::fixture()->entry('items', 'testo')['respawn'] ?? 'none', $store->get('items', 'testo')['respawn'] ?? 'none', 'the others keep the file\'s rule');
        self::assertSame([['product' => 'scorpion_amulet'], ['product' => 'dago_hoodie', 'variant' => 'black']], $store->list('featured'));
        self::assertSame(['scorpion_amulet', 'dago_hoodie:black'], array_column($store->rows('featured'), 'id'));
        self::assertNull($this->db->value("SELECT 1 FROM settings WHERE `key` IN ('item_respawn', 'featured_products')"), 'the settings are retired');
        self::assertSame(27, count($store->rows('items')));
    }

    public function testSlotsAndStatsAreTheContentPackagesConstants(): void
    {
        $items = (string) file_get_contents(dirname(self::root()) . '/shared/content/src/items.ts');
        self::assertSame(1, preg_match('/export const AVATAR_SLOTS: AvatarSlot\[\] = \[(.*?)\];/s', $items, $m));
        self::assertSame(AvatarSlots::ALL, array_map(fn($s) => trim($s, " \t\n\r'"), array_values(array_filter(explode(',', $m[1] ?? ''), fn($s) => trim($s) !== ''))));
        $substances = (string) file_get_contents(dirname(self::root()) . '/shared/content/src/substances.ts');
        self::assertSame(1, preg_match('/export const READER_STATS: Record<string, StatDef> = \{(.*?)\n\};/s', $substances, $m));
        preg_match_all("/(\w+): \{ label: \{ de: '([^']*)', en: '([^']*)' \}, baseline: (\d+), half_life_days: (\d+) \}/", $m[1] ?? '', $rows, PREG_SET_ORDER);
        $ts = [];
        foreach ($rows as $r) $ts[$r[1]] = ['label' => ['de' => $r[2], 'en' => $r[3]], 'baseline' => (int) $r[4], 'half_life_days' => (int) $r[5]];
        self::assertSame(ReaderStats::DEFINITIONS, $ts);
        $substances = (string) file_get_contents(dirname(self::root()) . '/shared/content/src/substances.ts');
        self::assertSame(1, preg_match('/export const READER_EFFECTS = \[(.*?)\] as const;/s', $substances, $m));
        self::assertSame(CatalogValidator::READER_EFFECTS, array_map(fn($s) => trim(trim($s), "'"), array_values(array_filter(explode(',', $m[1] ?? ''), fn($s) => trim($s) !== ''))));
        $fx = (string) file_get_contents(dirname(self::root()) . '/shared/content/src/fx.ts');
        self::assertSame(1, preg_match('/export const FX_SOUNDS = \[(.*?)\] as const;/s', $fx, $m));
        self::assertSame(CatalogValidator::FX_SOUNDS, array_map(fn($s) => trim(trim($s), "'"), array_values(array_filter(explode(',', $m[1] ?? ''), fn($s) => trim($s) !== ''))));
        self::assertArrayNotHasKey('slots', json_decode(self::config('items.json'), true), 'code, not catalog data');
        self::assertArrayNotHasKey('stats', json_decode(self::config('items.json'), true));
    }

    public function testTheSeedIsValidAndBadEntriesAreRefused(): void
    {
        $store = $this->get(CatalogStore::class);
        $validator = $this->get(CatalogValidator::class);
        foreach (array_keys(CatalogStore::KINDS) as $kind) {
            foreach ($store->rows($kind) as $row) {
                self::assertSame([], $validator->problems($kind, $row['id'], json_decode($row['data'], true)), "$kind {$row['id']}");
            }
        }
        $camera = $store->get('items', 'camera');
        self::assertIsArray($camera);
        $cases = [
            ['items', 'camera', ['slot' => 'tail'] + $camera, 'unknown slot "tail"'],
            ['items', 'camera', ['id' => 'other'] + $camera, 'data.id must be "camera"'],
            ['items', 'camera', ['edition_id' => 'nope'] + $camera, 'unknown edition "nope"'],
            ['items', 'camera', ['respawn' => ['default' => 'daily']] + $camera, 'respawn.default'],
            ['items', 'testo', ['effects' => [['stat' => 'mood', 'delta' => 5]]] + (array) $store->get('items', 'testo'), 'unknown stat "mood"'],
            ['items', 'bad id!', ['id' => 'bad id!'] + $camera, 'not a valid items id'],
            ['items', 'cucumber', ['grantable' => ['story']] + (array) $store->get('items', 'cucumber'), 'grants it by scan'],
            ['recipes', 'r', ['id' => 'r', 'inputs' => [['item' => 'ghost', 'qty' => 1]], 'output' => ['item' => 'camera', 'qty' => 0]], 'unknown item "ghost"'],
            ['items', 'camera', ['holds' => ['max' => 5]] + $camera, 'holds needs categories or items'],
            ['items', 'camera', ['holds' => ['categories' => ['score'], 'max' => 5, 'overfill' => ['days' => 3, 'max' => 4, 'breaks_into' => 'ghost']]] + $camera, 'overfill.max must be above holds.max'],
            ['items', 'camera', ['ages' => ['tint_days' => 100, 'spots_days' => 1, 'full_days' => 90]] + $camera, 'ages must be'],
            ['items', 'camera', ['score' => ['language' => 'en', 'musicxml' => 'x.pdf']] + $camera, 'score must be'],
            ['recipes', 'r', ['id' => 'r', 'inputs' => [['item' => 'camera', 'qty' => 1, 'keep' => true]], 'output' => ['item' => 'cucumber', 'qty' => 1]], 'use up at least one input'],
            ['recipes', 'r', ['id' => 'r', 'inputs' => [['item' => 'camera', 'qty' => 1]], 'output' => ['item' => 'cucumber', 'qty' => 1, 'variant_from' => 'cucumber']], 'variant_from must name one of the inputs'],
            ['products', 'dago_hoodie', ['item_id' => 'ghost'] + (array) $store->get('products', 'dago_hoodie'), 'unknown item "ghost"'],
            ['products', 'dago_hoodie', ['variants' => []] + (array) $store->get('products', 'dago_hoodie'), 'is featured'],
            ['featured', 'dago_hoodie:pink', ['product' => 'dago_hoodie', 'variant' => 'pink'], 'unknown variant pink'],
            ['code-types', 'membership', ['claimable' => true, 'signup' => true, 'grants' => ['root']], 'unknown permission "root"'],
            ['toons', 'newtoon', ['id' => 'newtoon', 'name' => 'New', 'redirectUrl' => 'javascript:alert(1)'], 'redirectUrl'],
            ['toons', 'New', ['id' => 'New', 'name' => 'New', 'redirectUrl' => 'https://scutoons.com/new'], 'not a valid toons id'],
            ['permissions', 'catalog', '', 'non-empty string'],
            ['albums', 'awakening', ['tracks' => [['id' => 'x', 'title' => 'X', 'src' => 'x.txt']]] + (array) $store->get('albums', 'awakening'), 'audio path'],
            ['meta', 'products', ['version' => 2, 'currency' => 'euro'], 'currency'],
        ];
        $admin = $this->get(CatalogAdmin::class);
        foreach ($cases as [$kind, $id, $data, $expect]) {
            try {
                $admin->put($kind, $id, $data, self::BY);
                self::fail("$kind $id: expected $expect");
            } catch (ApiError $error) {
                self::assertSame([422, 'invalid_catalog'], [$error->status, $error->errorCode], "$kind $id");
                self::assertStringContainsString($expect, $error->getMessage(), "$kind $id");
            }
        }
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM catalog_history'), 'nothing written');
    }

    public function testHistoryAndUndo(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $store = $this->get(CatalogStore::class);
        $catalog = $this->get(Catalog::class);
        $camera = json_decode((string) ($store->row('items', 'camera')['data'] ?? ''));
        $camera->name->en = 'Old camera';
        $update = $admin->put('items', 'camera', $camera, self::BY);
        self::assertSame([false, true], [$update['created'], $update['changed']]);
        self::assertSame('Old camera', $catalog->item('camera')['name']['en'] ?? null, 'readers in the same request see the change');
        self::assertFalse($admin->put('items', 'camera', $camera, self::BY)['changed'], 'unchanged');
        // a new item, at the end; ids are case-sensitive
        $new = (object) ['id' => 'Camera', 'toon' => 'catpope', 'name' => (object) ['en' => 'Big camera'], 'kind' => 'virtual', 'category' => 'tool', 'wear' => new \stdClass()];
        $create = $admin->put('items', 'Camera', $new, self::BY);
        self::assertSame([true, 27], [$create['created'], $create['entry']['position']]);
        self::assertStringContainsString('"wear":{}', (string) ($store->row('items', 'Camera')['data'] ?? ''), 'objects stay objects');
        self::assertSame(28, count($catalog->items()));
        // in use → 409; unused → deleted, and the delete can be undone (same place)
        try {
            $admin->delete('items', 'camera', self::BY);
            self::fail('the camera is in recipes');
        } catch (ApiError $error) {
            self::assertSame([409, 'in_use'], [$error->status, $error->errorCode]);
            self::assertContains('recipe cucumber_camera', $error->extra['uses'] ?? []);
        }
        $delete = $admin->delete('items', 'Camera', self::BY);
        self::assertNull($catalog->item('Camera'));
        $undo = $admin->undo((int) $delete['history_id'], self::BY);
        self::assertTrue($undo['undone']);
        self::assertSame(27, $store->row('items', 'Camera')['position'] ?? null);
        // undo the name change: back to the seed's
        $undo = $admin->undo((int) $update['history_id'], self::BY);
        self::assertSame('Camera', $catalog->item('camera')['name']['en'] ?? null);
        // the undo is a change too – undoing it again is a redo; an older change can't be undone over a newer one
        try {
            $admin->undo((int) $update['history_id'], self::BY);
            self::fail('changed since');
        } catch (ApiError $error) {
            self::assertSame([409, 'changed_since'], [$error->status, $error->errorCode]);
        }
        $admin->undo((int) $undo['history_id'], self::BY);
        self::assertSame('Old camera', $catalog->item('camera')['name']['en'] ?? null);
        // reorder + undo
        $before = array_column($store->rows('editions'), 'id');
        $reorder = $admin->reorder('editions', ['3_catpope_vinyl'], self::BY);
        self::assertSame(['3_catpope_vinyl', '1_catpope_dago', '2_catpope_beetles'], $reorder['ids']);
        self::assertSame('3_catpope_vinyl', $this->get(Editions::class)->all()[0]['id']);
        $admin->undo((int) $reorder['history_id'], self::BY);
        self::assertSame($before, array_column($store->rows('editions'), 'id'));

        $history = $admin->history('items', null);
        self::assertSame(['update', 'update', 'create', 'delete', 'create', 'update'], array_column($history, 'action'), 'newest first');
        self::assertSame([self::BY, 'camera'], [$history[0]['by'], $history[0]['entry_id']]);
        self::assertSame((int) $update['history_id'], $history[1]['undo_of']);
        self::assertSame('Camera', $history[5]['before']->name->en ?? null);
        self::assertCount(2, $admin->history('editions', null));
    }

    public function testAdminsCreateToonsTheCreatorThenFills(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $user = $this->user('boss@example.com');
        $result = $admin->put('toons', 'newtoon', (object) ['id' => 'newtoon', 'name' => 'New Toon', 'redirectUrl' => 'https://scutoons.com/newtoon'], self::BY, (string) $user['id']);
        self::assertTrue($result['created']);
        self::assertSame('New Toon', $this->get(Toons::class)->find('newtoon')['name'] ?? null);
        $entry = $this->get(EntityStore::class)->latest('toon:newtoon');
        self::assertSame(['draft', (string) $user['id']], [$entry['status'] ?? null, $entry['author'] ?? null]);
        $created = json_decode((string) ($entry['json'] ?? ''), true);
        self::assertArrayNotHasKey('i18n', $created, 'the name is the title (decision 4)');
        self::assertSame([['de' => 'New Toon', 'en' => 'New Toon'], ['de' => 'draft', 'en' => 'draft']], [$created['title'], $created['languageStatus']],
            'the toon settings per language in the entry (agents/NEUTRAL.md step 4)');
        $library = $this->get(Library::class);
        self::assertContains('newtoon', array_column(json_decode($library->toons(true)->body, true), 'id'), 'the creator lists it (drafts)');
        self::assertNotContains('newtoon', array_column(json_decode($library->toons(false)->body, true), 'id'), 'readers don\'t');
        // a toon with content can't be deleted; an existing index entry is kept when the toon is created again
        try {
            $admin->delete('toons', 'newtoon', self::BY);
            self::fail('content exists');
        } catch (ApiError $error) {
            self::assertSame('in_use', $error->errorCode);
        }
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE entity_key = 'toon:newtoon'"));
        self::assertNull($this->get(Toons::class)->find('sandbox'), 'the sandbox is content only (dev), never a QR toon');
    }

    public function testFeaturedIsACatalogWrite(): void
    {
        $featured = $this->get(FeaturedProducts::class);
        self::assertTrue($featured->update([['product' => 'scorpion_amulet']], self::BY));
        self::assertSame('[{"product":"scorpion_amulet"}]', $featured->json());
        self::assertSame(self::BY, $featured->state()['updated_by']);
        $replace = $this->get(CatalogAdmin::class)->history('featured', null)[0];
        self::assertSame('replace', $replace['action']);
        $this->get(CatalogAdmin::class)->undo($replace['id'], self::BY);
        self::assertSame(self::canonical(self::config('featured-products.json')), self::canonical($featured->json()));
        self::assertTrue($featured->update([], self::BY), 'an empty list = nothing featured (there is no default list)');
        self::assertSame('[]', $featured->json());
        try {
            $featured->update(null, self::BY);
            self::fail('null is not a list');
        } catch (ApiError $error) {
            self::assertSame('invalid_featured', $error->errorCode);
        }
    }
}
