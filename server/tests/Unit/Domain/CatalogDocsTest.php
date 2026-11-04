<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * The catalog entries as versioned documents (Domain\Catalog\CatalogDocs, agents/CATALOG-IN-CREATOR.md): drafts through
 * /api/entities, publish writes the catalog table, physical changes only through the `physical` review rule (owner
 * 2026-10-12), admin writes kept in step.
 */
final class CatalogDocsTest extends TestCase
{
    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
        $this->container->set(HttpClient::class, $this->core);
    }

    /**
     * @param array<string, mixed> $fields
     * @return array<string, mixed> a new virtual item
     */
    private static function item(string $id, array $fields = []): array
    {
        return $fields + ['id' => $id, 'toon' => 'catpope', 'name' => ['de' => 'Ding', 'en' => 'Thing'], 'description' => ['de' => '', 'en' => ''],
            'kind' => 'virtual', 'category' => 'collectible', 'icon' => null, 'stackable' => false, 'starter' => false, 'grantable' => ['story'],
            'slot' => null, 'skills' => [], 'edition_id' => null, 'product' => null, 'variants' => null];
    }

    public function testEveryEntryHasItsPublishedDocumentAndDraftsStayOutOfTheCatalog(): void
    {
        $writer = $this->creator('writer@example.com');
        // the migration gave every entry its first version
        $camera = $this->get(EntityStore::class)->published('item:camera');
        self::assertNotNull($camera);
        self::assertSame($this->db->value("SELECT data FROM catalog_items WHERE id = 'camera'"), $camera['json']);

        // a new item: a draft, not in the catalog until it is published
        self::assertSame(200, $this->put($writer, 'item:spoon', self::item('spoon'))->getStatusCode());
        self::assertFalse($this->inCatalog('items', 'spoon'));
        $list = self::json($this->request('GET', '/api/catalog/docs/items', $this->bearer($writer)));
        $spoon = array_values(array_filter($list['entries'], fn($e) => $e['id'] === 'spoon'))[0];
        self::assertSame([false, true, false, ['catpope']], [$spoon['live'], $spoon['draft'], $spoon['physical'], $spoon['toons']]);
        self::assertSame('camera', $list['entries'][0]['id'], 'the table order first');

        $published = $this->request('POST', '/api/entities/item:spoon/publish', $this->bearer($writer), '{}');
        self::assertSame(200, $published->getStatusCode(), (string) $published->getBody());
        self::assertSame('Thing', $this->data('items', 'spoon')['name']['en']);
        self::assertSame('published v' . self::json($published)['version']['id'], $this->db->value("SELECT note FROM catalog_history WHERE entry_id = 'spoon' ORDER BY id DESC LIMIT 1"));

        // a draft is checked as a catalog entry; the id is the key's
        $bad = $this->put($writer, 'item:fork', self::item('knife'));
        self::assertSame([422, 'invalid_catalog'], [$bad->getStatusCode(), self::json($bad)['error'] ?? null]);
        // a recipe may name an item that is only a draft so far – publishing it waits for the item
        $this->put($writer, 'item:bowl', self::item('bowl'));
        $recipe = ['id' => 'soup', 'inputs' => [['item' => 'cucumber', 'qty' => 1]], 'output' => ['item' => 'bowl', 'qty' => 1]];
        self::assertSame(200, $this->put($writer, 'recipe:soup', $recipe)->getStatusCode());
        $early = $this->request('POST', '/api/entities/recipe:soup/publish', $this->bearer($writer), '{}');
        self::assertSame([422, 'invalid_catalog'], [$early->getStatusCode(), self::json($early)['error'] ?? null]);
        self::assertSame('draft', $this->db->value("SELECT status FROM entity_versions WHERE entity_key = 'recipe:soup' ORDER BY id DESC LIMIT 1"), 'rolled back');
        $this->request('POST', '/api/entities/item:bowl/publish', $this->bearer($writer), '{}');
        self::assertSame(200, $this->request('POST', '/api/entities/recipe:soup/publish', $this->bearer($writer), '{}')->getStatusCode());

        // no unpublish – an entry leaves the catalog through the admin
        $un = $this->request('POST', '/api/entities/item:spoon/unpublish', $this->bearer($writer), '{}');
        self::assertSame([422, 'not_supported'], [$un->getStatusCode(), self::json($un)['error'] ?? null]);
    }

    /** research/CAVE.md: what homes look like – a document kind of its own, edited in the creator like albums */
    public function testHomeScenesAreDocumentsToo(): void
    {
        $writer = $this->creator('writer@example.com');
        $list = self::json($this->request('GET', '/api/catalog/docs/home-scenes', $this->bearer($writer)));
        self::assertSame(['cave', 'tatami_room', 'cathedral'], array_column($list['entries'], 'id'), 'the scenes of the migrations (homes, the cathedral set)');
        $scene = ['id' => 'garden_shed', 'name' => ['de' => 'Gartenhütte', 'en' => 'Garden shed'], 'image' => ['src' => 'scutoons/homes/garden_shed.1080p.webp', 'variants' => ['high' => 'scutoons/homes/garden_shed.4k.webp']], 'video' => ['src' => 'scutoons/homes/garden_shed.1080p.mp4']];
        self::assertSame(200, $this->put($writer, 'home-scene:garden_shed', $scene)->getStatusCode());
        self::assertFalse($this->inCatalog('home-scenes', 'garden_shed'));
        self::assertSame(200, $this->request('POST', '/api/entities/home-scene:garden_shed/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertSame('Garden shed', $this->data('home-scenes', 'garden_shed')['name']['en']);
        $bad = $this->put($writer, 'home-scene:attic', ['id' => 'attic', 'name' => ['en' => 'Attic'], 'image' => ['src' => ''], 'video' => ['src' => 'attic.gif']]);
        self::assertSame([422, 'invalid_catalog'], [$bad->getStatusCode(), self::json($bad)['error'] ?? null]);
    }

    /** #258: instruments, presets and rigs are documents too – `preset:<a.b>`, `rig:<subject>` (ids with dots) */
    public function testRigKindsAreDocumentsToo(): void
    {
        $writer = $this->creator('writer@example.com');
        // the core (#313, a fake): a preset whose parent did not come along with it is refused in the core's words
        $this->core->answer = function (array $r): array {
            $parent = $r['document']['parent'] ?? null;
            if ($r['kind'] === 'presets' && is_string($parent) && !in_array($parent, array_column($r['presets'], 'id'), true)) return ["preset {$r['id']}: unknown preset \"$parent\" (its parent)"];
            if ($r['kind'] === 'presets' && isset($r['document']['values']['drone.cv'])) return ['instrument drone: drone.cv is not a control input of osc'];
            return [];
        };
        $rigs = self::json($this->request('GET', '/api/catalog/docs/rigs', $this->bearer($writer)));
        self::assertSame(['character.catpope', 'item.incense_burner'], array_column($rigs['entries'], 'id'), 'the migrated core rigs (table data, no document yet)');
        // a derivative whose parent is only a draft so far: saved, and published after its parent
        self::assertSame(200, $this->put($writer, 'preset:drone.dark', ['id' => 'drone.dark', 'instrument' => 'drone', 'parent' => 'drone.base', 'values' => ['drone.freq' => 41.0]])->getStatusCode());
        self::assertSame(200, $this->put($writer, 'preset:drone.dark.deep', ['id' => 'drone.dark.deep', 'instrument' => 'drone', 'parent' => 'drone.dark', 'values' => ['drone.freq' => 30.0]])->getStatusCode());
        $early = $this->request('POST', '/api/entities/preset:drone.dark.deep/publish', $this->bearer($writer), '{}');
        self::assertSame([422, 'invalid_catalog', ['preset drone.dark.deep: unknown preset "drone.dark" (its parent)']], [$early->getStatusCode(), self::json($early)['error'] ?? null, self::json($early)['problems'] ?? null]);
        self::assertSame(200, $this->request('POST', '/api/entities/preset:drone.dark/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/entities/preset:drone.dark.deep/publish', $this->bearer($writer), '{}')->getStatusCode());
        $rig = ['subject' => 'character.boy', 'track' => [['id' => 'pad', 'instrument' => 'drone', 'preset' => 'drone.dark.deep']], 'moods' => ['sad' => (object) []]];
        self::assertSame(200, $this->put($writer, 'rig:character.boy', $rig)->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/entities/rig:character.boy/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertStringContainsString('"moods":{"sad":{}}', (string) $this->db->value("SELECT data FROM catalog_rigs WHERE id = 'character.boy'"), '{} stays {} (serde\'s Variant)');
        // what the core refuses: saved as a draft (the creator's hints say why), refused on publish in the core's words
        self::assertSame(200, $this->put($writer, 'preset:drone.bad', ['id' => 'drone.bad', 'instrument' => 'drone', 'values' => ['drone.cv' => 1.0]])->getStatusCode());
        $bad = $this->request('POST', '/api/entities/preset:drone.bad/publish', $this->bearer($writer), '{}');
        self::assertSame([422, 'invalid_catalog', ['instrument drone: drone.cv is not a control input of osc']], [$bad->getStatusCode(), self::json($bad)['error'] ?? null, self::json($bad)['problems'] ?? null]);
        // the catalog's own rule holds for a draft too
        self::assertSame(422, $this->put($writer, 'rig:character.girl', ['subject' => 'character.boy', 'track' => []])->getStatusCode(), 'subject = the id');
    }

    public function testAnAdminWriteBecomesThePublishedVersion(): void
    {
        $admin = $this->get(CatalogAdmin::class);
        $store = $this->get(EntityStore::class);
        $data = json_decode((string) $this->db->value("SELECT data FROM catalog_items WHERE id = 'camera'"));
        $data->name->en = 'Old camera';
        $admin->put('items', 'camera', $data, 'admin:boss@example.com');
        $published = (array) $store->published('item:camera');
        self::assertSame('Old camera', json_decode((string) $published['json'], true)['name']['en']);
        self::assertStringContainsString('admin:boss@example.com', (string) $published['message']);
        $count = (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE entity_key = 'item:camera'");
        $admin->put('items', 'camera', $data, 'admin:boss@example.com');
        self::assertSame($count, (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE entity_key = 'item:camera'"), 'unchanged: no version');
        // a delete leaves a tombstone as the published version (D5: its last document + removed: true, the id stays taken)
        $admin->put('items', 'ladle', self::item('ladle'), 'admin:boss@example.com');
        $admin->delete('items', 'ladle', 'admin:boss@example.com');
        self::assertTrue(json_decode((string) ($store->published('item:ladle')['json'] ?? '{}'), true)['removed'] ?? false);
    }

    /** owner 2026-10-12: physical items are reviewed – "damit nicht aus Versehen auf der Plattform was rausgesetzt werden kann" */
    public function testPhysicalChangesNeedThePhysicalRule(): void
    {
        $writer = $this->creator('writer@example.com');
        $editor = $this->creator('editor@example.com');
        $keeper = $this->creator('keeper@example.com');
        $reviews = $this->get(Reviews::class);
        $reviews->setRule((string) $editor['id'], 'catpope', Reviews::REVIEWER, true);

        // a virtual item's text: no rule → published directly
        $this->put($writer, 'item:cucumber', array_merge($this->data('items', 'cucumber'), ['name' => ['de' => 'Gurke', 'en' => 'Pickle']]));
        self::assertSame(200, $this->request('POST', '/api/entities/item:cucumber/publish', $this->bearer($writer), '{}')->getStatusCode());

        // an edition on a virtual item (every unclaimed code of it would offer the item) is physical
        $this->put($writer, 'item:spoon', self::item('spoon', ['edition_id' => '1_catpope_dago']));
        $denied = $this->request('POST', '/api/entities/item:spoon/publish', $this->bearer($writer), '{}');
        self::assertSame([403, 'review_required', true], [$denied->getStatusCode(), self::json($denied)['error'] ?? null, self::json($denied)['physical'] ?? null]);
        self::assertFalse($this->inCatalog('items', 'spoon'));
        // a physical item's description is physical too (before or after counts)
        $hoodie = $this->data('items', 'dago_hoodie');
        $this->put($writer, 'item:dago_hoodie', array_merge($hoodie, ['description' => ['de' => 'Warm', 'en' => 'Warm']]));
        self::assertSame(403, $this->request('POST', '/api/entities/item:dago_hoodie/publish', $this->bearer($editor), '{}')->getStatusCode(), 'a content reviewer is not enough');

        // nobody holds the physical rule yet: a submit has nobody to go to
        $submit = fn() => $this->request('POST', '/api/reviews', $this->bearer($writer), '{"items":[{"key":"item:spoon"},{"key":"item:dago_hoodie"}],"note":"merch"}');
        $none = $submit();
        self::assertSame([409, 'no_reviewer', 'physical'], [$none->getStatusCode(), self::json($none)['error'] ?? null, self::json($none)['rule'] ?? null]);
        $reviews->setRule((string) $keeper['id'], 'catpope', Reviews::PHYSICAL, true);
        $sent = $submit();
        self::assertSame(200, $sent->getStatusCode(), (string) $sent->getBody());
        self::assertSame(['keeper@example.com'], self::json($sent)['reviewers']);
        $ids = array_column(self::json($sent)['reviews'], 'id');

        // the content reviewer neither sees nor decides them; the physical one does
        self::assertSame([], self::json($this->request('GET', '/api/reviews', $this->bearer($editor)))['to_review']);
        self::assertSame(403, $this->request('POST', '/api/reviews/publish', $this->bearer($editor), json_encode(['ids' => $ids]) ?: '')->getStatusCode());
        $overview = self::json($this->request('GET', '/api/reviews', $this->bearer($keeper)));
        self::assertSame([$ids, ['catpope']], [array_column($overview['to_review'], 'id'), $overview['physical_of']]);
        $done = $this->request('POST', '/api/reviews/publish', $this->bearer($keeper), json_encode(['ids' => $ids]) ?: '');
        self::assertSame(200, $done->getStatusCode(), (string) $done->getBody());
        self::assertSame('1_catpope_dago', $this->db->value("SELECT edition_id FROM catalog_items WHERE id = 'spoon'"));
        self::assertSame('Warm', $this->data('items', 'dago_hoodie')['description']['en']);

        // the physical rule publishes its own physical changes directly; products and editions are always physical
        $product = $this->data('products', 'dago_hoodie');
        $this->put($keeper, 'product:dago_hoodie', array_merge($product, ['price' => 99]));
        self::assertSame(403, $this->request('POST', '/api/entities/product:dago_hoodie/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/entities/product:dago_hoodie/publish', $this->bearer($keeper), '{}')->getStatusCode());
        // a cross-toon item (toon null) answers to the `*` rules only
        $this->put($writer, 'item:mug', self::item('mug', ['toon' => null, 'max_world' => 1]));
        self::assertSame(403, $this->request('POST', '/api/entities/item:mug/publish', $this->bearer($keeper), '{}')->getStatusCode(), 'catpope\'s physical rule does not cover every toon');
        $reviews->setRule((string) $keeper['id'], Reviews::ALL_TOONS, Reviews::PHYSICAL, true);
        self::assertSame(200, $this->request('POST', '/api/entities/item:mug/publish', $this->bearer($keeper), '{}')->getStatusCode());
        self::assertSame('item spoon, item dago_hoodie', Reviews::files(['item:spoon', 'item:dago_hoodie']));
    }

    /** D6: the admin's five catalog tabs are read-only – their entries are edited in the creator; the order stays there */
    public function testTheAdminNoLongerWritesTheMovedKinds(): void
    {
        $boss = $this->creator('boss@example.com');
        $this->get(Roles::class)->grant((string) $boss['id'], Roles::ADMIN);
        $camera = $this->data('items', 'camera');
        $put = $this->request('PUT', '/api/admin/catalog/items/camera', $this->bearer($boss), (string) json_encode(['data' => array_merge($camera, ['edition_id' => '1_catpope_dago'])]));
        self::assertSame([409, 'edited_in_creator'], [$put->getStatusCode(), self::json($put)['error'] ?? null]);
        self::assertNull($this->data('items', 'camera')['edition_id']);
        self::assertSame(409, $this->request('DELETE', '/api/admin/catalog/editions/1_catpope_dago', $this->bearer($boss))->getStatusCode());
        self::assertSame(200, $this->request('GET', '/api/admin/catalog/items/camera', $this->bearer($boss))->getStatusCode());
        self::assertSame(200, $this->request('POST', '/api/admin/catalog/items/reorder', $this->bearer($boss), '{"ids":["cucumber"]}')->getStatusCode());
        // the shop & codes kinds stay the admin's
        self::assertSame(201, $this->request('PUT', '/api/admin/catalog/permissions/test_perm', $this->bearer($boss), '{"data":"A test permission"}')->getStatusCode());
    }

    /** D5 (as the map's places): never published → deleted; published → a tombstone, publishing it takes the entry out */
    public function testRemovingAnEntry(): void
    {
        $writer = $this->creator('writer@example.com');
        $this->put($writer, 'item:spoon', self::item('spoon'));
        $gone = $this->request('POST', '/api/entities/batch', $this->bearer($writer), '{"deletes":["item:spoon"]}');
        self::assertSame(200, $gone->getStatusCode(), (string) $gone->getBody());
        self::assertNull($this->get(EntityStore::class)->latest('item:spoon'), 'never published: deleted, the id is free');
        $refused = $this->request('POST', '/api/entities/batch', $this->bearer($writer), '{"deletes":["item:tuna_can"]}');
        self::assertSame([409, 'published'], [$refused->getStatusCode(), self::json($refused)['error'] ?? null]);

        // a published, unused entry: its last document + removed: true, then publish → out of the catalog, id taken
        $this->put($writer, 'item:fork', self::item('fork'));
        $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}');
        self::assertSame(200, $this->put($writer, 'item:fork', self::item('fork', ['removed' => true]))->getStatusCode(), 'a tombstone is not checked again');
        self::assertTrue($this->inCatalog('items', 'fork'), 'a draft tombstone changes nothing yet');
        self::assertSame(200, $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertFalse($this->inCatalog('items', 'fork'));
        $fork = array_values(array_filter(self::json($this->request('GET', '/api/catalog/docs/items', $this->bearer($writer)))['entries'], fn($e) => $e['id'] === 'fork'))[0];
        self::assertSame([true, false, false], [$fork['removed'], $fork['live'], $fork['draft']]);
        self::assertSame(409, $this->request('POST', '/api/entities/batch', $this->bearer($writer), '{"deletes":["item:fork"]}')->getStatusCode(), 'its id stays taken');

        // a used entry can't leave (409 in_use, nothing changes); a physical one needs the physical rule
        $this->put($writer, 'item:cucumber', array_merge($this->data('items', 'cucumber'), ['removed' => true]));
        $used = $this->request('POST', '/api/entities/item:cucumber/publish', $this->bearer($writer), '{}');
        self::assertSame([409, 'in_use'], [$used->getStatusCode(), self::json($used)['error'] ?? null]);
        self::assertTrue($this->inCatalog('items', 'cucumber'));
        $this->put($writer, 'item:awakening_test_press', array_merge($this->data('items', 'awakening_test_press'), ['removed' => true]));
        self::assertSame(403, $this->request('POST', '/api/entities/item:awakening_test_press/publish', $this->bearer($writer), '{}')->getStatusCode());
    }

    /** a batch checks catalog documents like a single save */
    public function testABatchChecksCatalogDocuments(): void
    {
        $writer = $this->creator('writer@example.com');
        $bad = $this->request('POST', '/api/entities/batch', $this->bearer($writer), (string) json_encode(['saves' => [['key' => 'item:fork', 'content' => self::item('knife')]]]));
        self::assertSame([422, 'invalid_catalog', 'item:fork'], [$bad->getStatusCode(), self::json($bad)['error'] ?? null, self::json($bad)['key'] ?? null]);
    }

    /** D2: the owner gets the physical rule for every toon by migration – once, nobody else */
    public function testTheOwnersPhysicalRuleMigration(): void
    {
        $owner = $this->user('tilman@porschuetz.de');
        $other = $this->user('someone@example.com');
        $sql = (string) file_get_contents(self::root() . '/migrations/mysql/2026-10-12-owner-physical-rule.mysql.sql');
        $statement = trim((string) preg_replace('/^--.*$/m', '', $sql));
        $this->db->run($statement);
        $this->db->run($statement);
        self::assertSame([['*', 'physical']], array_map(fn($r) => [$r['toon'], $r['rule']], $this->db->all('SELECT toon, rule FROM content_review_rules WHERE user_id = ?', [$owner['id']])));
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM content_review_rules WHERE user_id = ?', [$other['id']]));
    }

    /** the rules combine per toon: grants and the restriction each count where they hold; `*` adds to a toon's own */
    public function testRulesCombine(): void
    {
        $writer = $this->creator('writer@example.com');
        $reviews = $this->get(Reviews::class);
        $id = (string) $writer['id'];
        // "Submits for review" + "Publishes physical items" in one toon: the physical change goes out, the story one waits
        $reviews->setRule($id, 'catpope', Reviews::NEEDS_REVIEW, true);
        $reviews->setRule($id, 'catpope', Reviews::PHYSICAL, true);
        $this->put($writer, 'item:mug', self::item('mug', ['edition_id' => '1_catpope_dago']));
        $this->put($writer, 'item:spoon', self::item('spoon'));
        self::assertSame(200, $this->request('POST', '/api/entities/item:mug/publish', $this->bearer($writer), '{}')->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/entities/item:spoon/publish', $this->bearer($writer), '{}')->getStatusCode());
        // `*` + the toon's own: either counts (the physical rule for every toon covers a cross-toon entry, catpope's doesn't)
        $this->put($writer, 'item:cup', self::item('cup', ['toon' => null, 'max_world' => 3]));
        self::assertSame(403, $this->request('POST', '/api/entities/item:cup/publish', $this->bearer($writer), '{}')->getStatusCode());
        $reviews->setRule($id, Reviews::ALL_TOONS, Reviews::PHYSICAL, true);
        $reviews->setRule($id, 'catpope', Reviews::PHYSICAL, false);
        self::assertSame(200, $this->request('POST', '/api/entities/item:cup/publish', $this->bearer($writer), '{}')->getStatusCode());
        $this->put($writer, 'item:mug', self::item('mug', ['edition_id' => '1_catpope_dago', 'name' => ['de' => 'Becher', 'en' => 'Mug']]));
        self::assertSame(200, $this->request('POST', '/api/entities/item:mug/publish', $this->bearer($writer), '{}')->getStatusCode(), '* covers catpope');
        // "Submits for review" on ANY toon covers cross-toon entries too, non-physical ones included (owner 2026-10-12);
        // their requests go to the `*` reviewers
        $this->put($writer, 'item:fork', self::item('fork', ['toon' => null]));
        $denied = $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}');
        self::assertSame([403, 'review_required'], [$denied->getStatusCode(), self::json($denied)['error'] ?? null]);
        $boss = $this->creator('boss@example.com');
        $reviews->setRule((string) $boss['id'], Reviews::ALL_TOONS, Reviews::REVIEWER, true);
        self::assertSame(['boss@example.com'], self::json($this->request('POST', '/api/reviews', $this->bearer($writer), '{"items":[{"key":"item:fork"}]}'))['reviewers']);
        $reviews->setRule($id, 'catpope', Reviews::NEEDS_REVIEW, false);
        self::assertSame(200, $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}')->getStatusCode(), 'no restriction left');
    }

    public function testNeedsReviewAppliesToTheCatalogOfThatToon(): void
    {
        $writer = $this->creator('writer@example.com');
        $editor = $this->creator('editor@example.com');
        $reviews = $this->get(Reviews::class);
        $reviews->setRule((string) $writer['id'], 'catpope', Reviews::NEEDS_REVIEW, true);
        $reviews->setRule((string) $editor['id'], 'catpope', Reviews::REVIEWER, true);
        $this->put($writer, 'item:spoon', self::item('spoon'));
        self::assertSame(403, $this->request('POST', '/api/entities/item:spoon/publish', $this->bearer($writer), '{}')->getStatusCode());
        // another toon's item is not catpope's; a cross-toon one acts on catpope too (owner 2026-10-12)
        $this->put($writer, 'item:knife', self::item('knife', ['toon' => 'yellowmouse']));
        self::assertSame(200, $this->request('POST', '/api/entities/item:knife/publish', $this->bearer($writer), '{}')->getStatusCode());
        $this->put($writer, 'item:fork', self::item('fork', ['toon' => null]));
        self::assertSame(403, $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}')->getStatusCode());
        $sent = self::json($this->request('POST', '/api/reviews', $this->bearer($writer), '{"items":[{"key":"item:spoon"}]}'));
        self::assertSame(['editor@example.com'], $sent['reviewers']);
        self::assertSame(200, $this->request('POST', '/api/reviews/publish', $this->bearer($editor), json_encode(['ids' => [$sent['reviews'][0]['id']]]) ?: '')->getStatusCode());
        self::assertTrue($this->inCatalog('items', 'spoon'));
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed> an entry as the table has it now (the requests run in their own container) */
    private function data(string $kind, string $id): array
    {
        $json = $this->db->value('SELECT data FROM ' . CatalogStore::table($kind) . ' WHERE id = ?', [$id]);
        if ($json === null) self::fail("no $kind entry $id");
        return (array) json_decode((string) $json, true);
    }

    private function inCatalog(string $kind, string $id): bool
    {
        return $this->db->value('SELECT 1 FROM ' . CatalogStore::table($kind) . ' WHERE id = ?', [$id]) !== null;
    }

    /** @return array<string, mixed> */
    private function creator(string $email): array
    {
        $user = $this->user($email);
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        return $user;
    }

    /**
     * @param array<string, mixed> $user
     * @param array<string, mixed> $content
     */
    private function put(array $user, string $key, array $content): ResponseInterface
    {
        return $this->request('PUT', "/api/entities/$key", $this->bearer($user), (string) json_encode(['content' => $content]));
    }

    /**
     * @param array<string, mixed> $user
     * @return array<string, string>
     */
    private function bearer(array $user): array
    {
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']), 'Content-Type' => 'application/json'];
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        $container->set(HttpClient::class, $this->core);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }
}
