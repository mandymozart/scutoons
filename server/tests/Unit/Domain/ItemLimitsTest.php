<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Content\ItemPlacements;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Item limits across the system (BACKLOG 10.39 step 7, ITEMS.md "Item limits"): a catalog item's `placements` (on
 * slices, every toon together) and `max_world` (accounts that own it at once).
 */
final class ItemLimitsTest extends TestCase
{
    /** @param array<mixed> $doc */
    private function store(string $key, array $doc, string $status = 'draft'): void
    {
        $id = $this->get(EntityStore::class)->insert(EntityKey::parse($key), (string) json_encode($doc, JSON_UNESCAPED_SLASHES), null, $status, null);
        if ($status === 'published') $this->db->run("UPDATE entity_versions SET status = 'draft' WHERE entity_key = ? AND id <> ? AND status = 'published'", [$key, $id]);
    }

    /** @return array<string, mixed> a chapter with one slice holding `$n` placements of `$item` */
    private static function chapter(int $number, string $item, int $n): array
    {
        $items = [];
        for ($i = 0; $i < $n; $i++) $items[] = ['id' => "$item-x$i", 'item' => $item, 'rect' => ['x' => 0, 'y' => 0, 'width' => 10]];
        return ['number' => $number, 'groups' => [['label' => 'a', 'slices' => [['image' => 'a.webp', 'width' => 390, 'height' => 625, 'items' => $items]]]]];
    }

    private function limit(string $item, string $field, int $value): void
    {
        $this->db->run("UPDATE catalog_items SET data = JSON_SET(data, ?, CAST(? AS SIGNED)) WHERE id = ?", ['$.' . $field, $value, $item]); // a bound int arrives as a string: CAST keeps it a JSON number
    }

    public function testPlacementsAreCountedOverEveryToonAndASaveMayNotAddOverTheLimit(): void
    {
        $this->limit('cucumber', 'placements', 3);
        $this->store('chapter:catpope:1', self::chapter(1, 'cucumber', 1), 'published');
        $this->store('chapter:catpope:1', self::chapter(1, 'cucumber', 0)); // a newer draft removes it: the published one counts
        $this->store('scene:toptoy:1:boss', ['label' => 'boss', 'slices' => [['image' => 'b.webp', 'items' => [['id' => 'c', 'item' => 'cucumber']]]]]);
        $this->store('chapter:catpope:1:de', self::chapter(1, 'cucumber', 5)); // a legacy per-locale document doesn't count
        $placements = $this->get(ItemPlacements::class);
        $report = $placements->report('cucumber');
        self::assertSame(['placements' => 3, 'max_world' => null], $report['limits']);
        self::assertSame(2, $report['total']);
        self::assertSame(['chapter:catpope:1', 'scene:toptoy:1:boss'], array_column($report['placements'], 'key'));

        $key = EntityKey::parse('chapter:sandbox:2');
        self::assertSame([], $placements->problems($key, json_decode((string) json_encode(self::chapter(2, 'cucumber', 1)))), '2 + 1 = 3: fine');
        $over = $placements->problems($key, json_decode((string) json_encode(self::chapter(2, 'cucumber', 2))));
        self::assertCount(1, $over);
        self::assertStringStartsWith('cucumber: 4 placements, the catalog allows 3 – also in chapter:catpope:1 (1), scene:toptoy:1:boss (1)', $over[0]);
        // a document already over the limit still saves as long as it doesn't add one
        $this->store('chapter:sandbox:2', self::chapter(2, 'cucumber', 2));
        self::assertSame([], $placements->problems($key, json_decode((string) json_encode(self::chapter(2, 'cucumber', 2)))));
        self::assertSame([], $placements->problems($key, json_decode((string) json_encode(self::chapter(2, 'camera', 9)))), 'no limit: anything goes');
        $this->expectException(ApiError::class);
        $placements->report('unicorn');
    }

    public function testTheSaveIsRefusedAndTheReportIsServed(): void
    {
        $this->limit('cucumber', 'placements', 1);
        $this->store('chapter:catpope:1', self::chapter(1, 'cucumber', 1), 'published');
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], 'creator');
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $call = function (string $method, string $path, ?array $body = null) use ($token): ResponseInterface {
            $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$path", ['REMOTE_ADDR' => '127.0.0.1'])
                ->withHeader('Authorization', "Bearer $token")->withHeader('Content-Type', 'application/json');
            if ($body !== null) {
                $request->getBody()->write((string) json_encode($body));
                $request->getBody()->rewind();
            }
            $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
            $container->set(Clock::class, $this->clock);
            return Kernel::app($container)->handle($request);
        };
        $report = $call('GET', '/api/catalog/items/cucumber/placements');
        self::assertSame(200, $report->getStatusCode(), (string) $report->getBody());
        $data = json_decode((string) $report->getBody(), true);
        self::assertSame(1, $data['total']);
        self::assertSame(0, $data['owners']);
        self::assertSame(404, $call('GET', '/api/catalog/items/unicorn/placements')->getStatusCode());

        $content = self::chapter(2, 'cucumber', 1) + ['title' => ['de' => 'Zwei'], 'author' => 'A', 'release' => ['de' => '2026-01-01'], 'description' => ['de' => ''], 'thumbnail' => ''];
        $content['groups'][0]['renderer'] = ['type' => 'vertical'];
        $refused = $call('PUT', '/api/entities/chapter:catpope:2', ['content' => $content]);
        self::assertSame(422, $refused->getStatusCode(), (string) $refused->getBody());
        self::assertSame('item_limit', json_decode((string) $refused->getBody(), true)['error'] ?? null, (string) $refused->getBody());
    }

    public function testMaxWorldRefusesAGrantWhileThatManyAccountsOwnIt(): void
    {
        $this->limit('cucumber', 'max_world', 1);
        $inventory = $this->get(InventoryService::class);
        $first = (string) $this->user('first@example.com')['id'];
        $second = (string) $this->user('second@example.com')['id'];
        self::assertSame(1, $inventory->grant($first, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'a']));
        self::assertSame(1, $inventory->grant($first, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'b']), 'its owner gets more (max per account still applies)');
        try {
            $inventory->grant($second, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'a']);
            self::fail('taken expected');
        } catch (ApiError $error) {
            self::assertSame('taken', $error->errorCode);
            self::assertSame(409, $error->status);
        }
        $import = $inventory->import($second, ['items' => [['item_id' => 'cucumber', 'qty' => 1]]]);
        self::assertSame('taken', $import['skipped'][0]['reason'] ?? null);
        $inventory->remove($first, 'cucumber', 2);
        self::assertSame(1, $inventory->grant($second, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'a']), 'back in the world: the next one may have it');
    }
}
