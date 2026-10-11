<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * The catalog entries as versioned documents (Domain\Catalog\CatalogDocs, agents/CATALOG-IN-CREATOR.md): drafts through
 * /api/entities, publish writes the catalog table, physical changes only through the `physical` review rule (owner
 * 2026-10-12), admin writes kept in step.
 */
final class CatalogDocsTest extends TestCase
{
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
        // a delete leaves the document unpublished
        $admin->put('items', 'ladle', self::item('ladle'), 'admin:boss@example.com');
        $admin->delete('items', 'ladle', 'admin:boss@example.com');
        self::assertNull($store->published('item:ladle'));
        self::assertNotNull($store->latest('item:ladle'));
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

    /** the admin's old catalog page answers to the same rule: an admin without the physical rule can't make an item claimable */
    public function testTheAdminPathNeedsThePhysicalRuleToo(): void
    {
        $boss = $this->creator('boss@example.com');
        $this->get(Roles::class)->grant((string) $boss['id'], Roles::ADMIN);
        $camera = $this->data('items', 'camera');
        $put = fn(array $data) => $this->request('PUT', '/api/admin/catalog/items/camera', $this->bearer($boss), (string) json_encode(['data' => $data]));
        self::assertSame(200, $put(array_merge($camera, ['name' => ['de' => 'Kamera', 'en' => 'Cam']]))->getStatusCode(), 'a virtual change: as before');
        $denied = $put(array_merge($camera, ['edition_id' => '1_catpope_dago']));
        self::assertSame([403, 'review_required'], [$denied->getStatusCode(), self::json($denied)['error'] ?? null]);
        self::assertNull($this->data('items', 'camera')['edition_id']);
        self::assertSame(403, $this->request('DELETE', '/api/admin/catalog/editions/1_catpope_dago', $this->bearer($boss))->getStatusCode());
        $this->get(Reviews::class)->setRule((string) $boss['id'], 'catpope', Reviews::PHYSICAL, true);
        self::assertSame(200, $put(array_merge($camera, ['edition_id' => '1_catpope_dago']))->getStatusCode());
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
        // another toon's / a cross-toon item is not catpope's
        $this->put($writer, 'item:fork', self::item('fork', ['toon' => null]));
        self::assertSame(200, $this->request('POST', '/api/entities/item:fork/publish', $this->bearer($writer), '{}')->getStatusCode());
        $sent = self::json($this->request('POST', '/api/reviews', $this->bearer($writer), '{"items":[{"key":"item:spoon"}]}'));
        self::assertSame(['editor@example.com'], $sent['reviewers']);
        self::assertSame(200, $this->request('POST', '/api/reviews/publish', $this->bearer($editor), json_encode(['ids' => [$sent['reviews'][0]['id']]]) ?: '')->getStatusCode());
        self::assertTrue($this->inCatalog('items', 'spoon'));
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────────────────────────────

    /** @return array<string, mixed> an entry as the table has it now (the requests run in their own container) */
    private function data(string $kind, string $id): array
    {
        $json = $this->db->value('SELECT data FROM catalog_' . $kind . ' WHERE id = ?', [$id]);
        if ($json === null) self::fail("no $kind entry $id");
        return (array) json_decode((string) $json, true);
    }

    private function inCatalog(string $kind, string $id): bool
    {
        return $this->db->value('SELECT 1 FROM catalog_' . $kind . ' WHERE id = ?', [$id]) !== null;
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
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }
}
