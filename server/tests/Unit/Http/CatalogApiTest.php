<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Avatar\AvatarSlots;
use Scutoons\Domain\Reader\ReaderStats;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Support\Json;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * The catalog over HTTP (agents/CATALOG.md): the public endpoints answer from the tables what they answered from the
 * config files; the admin API /api/admin/catalog/… (list / get / put / delete / reorder, history, undo, toons).
 */
final class CatalogApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
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

    /** @return array<string, string> */
    private function admin(): array
    {
        $user = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    private static function config(string $file): string
    {
        return (string) file_get_contents(self::root() . '/' . \Scutoons\Domain\Catalog\CatalogSeed::FIXTURE . "/$file");
    }

    public function testThePublicEndpointsAnswerTheFixture(): void
    {
        // the migrated tables = the fixture's files (+ slots / stats, which are code), decoded as arrays, the API's flags
        $file = json_decode(self::config('items.json'), true);
        $items = Json::encode(['version' => $file['version'], 'slots' => AvatarSlots::ALL, 'items' => $file['items'], 'recipes' => $file['recipes'], 'albums' => $file['albums'], 'stats' => ReaderStats::DEFINITIONS]);
        self::assertSame($items, (string) $this->request('GET', '/api/items/')->getBody());
        self::assertSame(json_decode($items, true), self::json($this->request('GET', '/api/items/items.json')), 'the old file URL');
        self::assertSame(Json::encode(json_decode(self::config('editions.json'), true)), (string) $this->request('GET', '/api/editions/')->getBody());
        self::assertSame('[]', (string) $this->request('GET', '/api/editions/?toon=toptoy')->getBody());
        $r = $this->request('GET', '/api/toons/');
        self::assertSame([(string) json_encode(json_decode(self::config('toons.json'), true)), 'application/json'], [(string) $r->getBody(), $r->getHeaderLine('Content-Type')], 'default flags, as since the first version');
        $catalog = self::json($this->request('GET', '/api/admin/catalog', $this->admin()));
        self::assertSame(json_decode(self::config('permissions.json'), true), $catalog['permissions']);
        self::assertSame(json_decode($items, true), $catalog['catalog']);
    }

    public function testAdminsEditTheCatalog(): void
    {
        self::assertSame(401, $this->request('GET', '/api/admin/catalog/items')->getStatusCode());
        $member = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user()['id'])];
        self::assertSame(403, $this->request('PUT', '/api/admin/catalog/toons/x', $member, '{"data":{}}')->getStatusCode());
        $auth = $this->admin();

        $list = self::json($this->request('GET', '/api/admin/catalog/items', $auth));
        self::assertSame([27, 'camera', 0], [count($list['items']), $list['items'][0]['id'], $list['items'][0]['position']]);
        self::assertSame(['membership', 'promo', 'page', 'event', 'xr', 'default'], array_column(self::json($this->request('GET', '/api/admin/catalog/code-types', $auth))['items'], 'id'));
        $entry = self::json($this->request('GET', '/api/admin/catalog/products/dago_hoodie', $auth))['entry'];
        self::assertSame(['dago_hoodie', 'beige', 'seed'], [$entry['data']['item_id'], $entry['data']['defaultVariant'], $entry['updated_by']]);
        self::assertSame(404, $this->request('GET', '/api/admin/catalog/items/nope', $auth)->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/admin/catalog/bogus', $auth)->getStatusCode());

        // create: 201, validated, objects kept
        $item = ['id' => 'onion', 'toon' => 'catpope', 'name' => ['de' => 'Zwiebel', 'en' => 'Onion'], 'kind' => 'virtual', 'category' => 'food', 'icon' => 'catpope/items/onion.webp',
            'stackable' => true, 'grantable' => ['story'], 'slot' => null, 'skills' => [], 'wear' => (object) []];
        $r = $this->request('PUT', '/api/admin/catalog/items/onion', $auth, (string) json_encode(['data' => $item]));
        $body = self::json($r);
        self::assertSame([201, true, true, 27], [$r->getStatusCode(), $body['created'], $body['changed'], $body['entry']['position']]);
        self::assertStringContainsString('"wear":{}', (string) $this->request('GET', '/api/admin/catalog/items/onion', $auth)->getBody(), 'stored as sent (GET /api/items/ decodes as arrays, as when it read the file)');
        $r = $this->request('PUT', '/api/admin/catalog/items/onion', $auth, (string) json_encode(['data' => ['slot' => 'tail'] + $item]));
        self::assertSame([422, 'invalid_catalog'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertStringContainsString('unknown slot "tail"', implode(' ', self::json($r)['problems']));
        self::assertSame([400, 'missing_data'], [$this->request('PUT', '/api/admin/catalog/items/onion', $auth, '{"item":1}')->getStatusCode(), self::json($this->request('PUT', '/api/admin/catalog/items/onion', $auth, '{"item":1}'))['error']]);
        // a recipe that uses it, then the item can't go
        $r = $this->request('PUT', '/api/admin/catalog/recipes/onion_soup', $auth, (string) json_encode(['data' => ['id' => 'onion_soup', 'inputs' => [['item' => 'onion', 'qty' => 2]], 'output' => ['item' => 'cucumber', 'qty' => 1]]]));
        self::assertSame(201, $r->getStatusCode());
        // a recipe's own picture must be an image path (refused = no history row)
        $r = $this->request('PUT', '/api/admin/catalog/recipes/onion_soup', $auth, (string) json_encode(['data' => ['id' => 'onion_soup', 'inputs' => [['item' => 'onion', 'qty' => 2]], 'output' => ['item' => 'cucumber', 'qty' => 1], 'image' => 'catpope/items/recipes/onion_soup.txt']]));
        self::assertSame([422, 'invalid_catalog'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertStringContainsString('image must be an image path', implode(' ', self::json($r)['problems']));
        $r = $this->request('DELETE', '/api/admin/catalog/items/onion', $auth);
        self::assertSame([409, 'in_use', ['recipe onion_soup']], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['uses']]);
        // reorder
        $r = $this->request('POST', '/api/admin/catalog/items/reorder', $auth, '{"ids":["onion","camera"]}');
        self::assertSame(['onion', 'camera'], array_slice(self::json($r)['ids'], 0, 2));
        self::assertSame('onion', self::json($this->request('GET', '/api/items/'))['items'][0]['id']);
        self::assertSame(400, $this->request('POST', '/api/admin/catalog/items/reorder', $auth, '{"ids":"onion"}')->getStatusCode());
        // history + undo (newest first)
        $history = self::json($this->request('GET', '/api/admin/catalog/history?limit=10', $auth))['items'];
        self::assertSame([['items', '', 'reorder'], ['recipes', 'onion_soup', 'create'], ['items', 'onion', 'create']], array_map(fn($h) => [$h['kind'], $h['entry_id'], $h['action']], $history));
        self::assertSame('admin:boss@example.com', $history[0]['by']);
        self::assertCount(1, self::json($this->request('GET', '/api/admin/catalog/history?kind=items&id=onion', $auth))['items']);
        $r = $this->request('POST', "/api/admin/catalog/history/{$history[0]['id']}/undo", $auth);
        self::assertSame([200, true], [$r->getStatusCode(), self::json($r)['undone']]);
        self::assertSame('camera', self::json($this->request('GET', '/api/items/'))['items'][0]['id']);
        $r = $this->request('POST', "/api/admin/catalog/history/{$history[2]['id']}/undo", $auth);
        self::assertSame([409, 'in_use'], [$r->getStatusCode(), self::json($r)['error']], 'undoing the item\'s creation = deleting it: the recipe still uses it');
        $this->request('POST', "/api/admin/catalog/history/{$history[1]['id']}/undo", $auth);
        self::assertSame(200, $this->request('POST', "/api/admin/catalog/history/{$history[2]['id']}/undo", $auth)->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/admin/catalog/items/onion', $auth)->getStatusCode());
        self::assertSame(404, $this->request('POST', '/api/admin/catalog/history/999999/undo', $auth)->getStatusCode());
    }

    public function testAcceptedWarningsAreStoredUndoneAndNeverServed(): void
    {
        // the admin's "Looks fine" (@scutoons/content catalogWarningPlace): accepted_warnings in the entry / meta items,
        // versioned like any change, left out of every reader answer
        $auth = $this->admin();
        $public = (string) $this->request('GET', '/api/items/')->getBody();
        $products = (string) $this->request('GET', '/api/products/')->getBody();
        $camera = self::json($this->request('GET', '/api/admin/catalog/items/camera', $auth))['entry']['data'];
        $r = $this->request('PUT', '/api/admin/catalog/items/camera', $auth, (string) json_encode(['data' => $camera + ['accepted_warnings' => ['category#1x2y3z']], 'note' => 'warning accepted']));
        self::assertSame([200, true], [$r->getStatusCode(), self::json($r)['changed']]);
        $meta = self::json($this->request('GET', '/api/admin/catalog/meta/items', $auth))['entry']['data'];
        self::assertSame(200, $this->request('PUT', '/api/admin/catalog/meta/items', $auth, (string) json_encode(['data' => $meta + ['accepted_warnings' => ['#abc']]]))->getStatusCode());
        $product = self::json($this->request('GET', '/api/admin/catalog/products/dago_hoodie', $auth))['entry']['data'];
        self::assertSame(200, $this->request('PUT', '/api/admin/catalog/products/dago_hoodie', $auth, (string) json_encode(['data' => $product + ['accepted_warnings' => ['price#q']]]))->getStatusCode());
        self::assertSame(['category#1x2y3z'], self::json($this->request('GET', '/api/admin/catalog/items/camera', $auth))['entry']['data']['accepted_warnings']);
        self::assertSame($public, (string) $this->request('GET', '/api/items/')->getBody(), 'readers never get accepted warnings');
        self::assertSame($products, (string) $this->request('GET', '/api/products/')->getBody());
        foreach ([['x'], [''], 'x', [1], []] as $bad) {
            $r = $this->request('PUT', '/api/admin/catalog/items/camera', $auth, (string) json_encode(['data' => ['accepted_warnings' => $bad] + $camera]));
            if ($bad === ['x']) {
                self::assertSame(200, $r->getStatusCode());
                continue;
            }
            self::assertSame([422, 'invalid_catalog'], [$r->getStatusCode(), self::json($r)['error']], (string) json_encode($bad));
        }
        // undo the last change of camera (['x']) and the acceptance before it: back to the stored item without the field
        $history = self::json($this->request('GET', '/api/admin/catalog/history?kind=items&id=camera&limit=5', $auth))['items'];
        self::assertSame('warning accepted', $history[1]['note']);
        foreach ([$history[0], $history[1]] as $h) self::assertSame(200, $this->request('POST', "/api/admin/catalog/history/{$h['id']}/undo", $auth)->getStatusCode());
        self::assertArrayNotHasKey('accepted_warnings', self::json($this->request('GET', '/api/admin/catalog/items/camera', $auth))['entry']['data']);
    }

    public function testAdminsCreateToons(): void
    {
        $auth = $this->admin();
        $r = $this->request('PUT', '/api/admin/catalog/toons/sandmouse', $auth, '{"data":{"id":"sandmouse","name":"Sand Mouse","redirectUrl":"https://scutoons.com/sandmouse"}}');
        self::assertSame(201, $r->getStatusCode());
        self::assertContains('sandmouse', array_column(self::json($this->request('GET', '/api/toons/')), 'id'), 'QR codes can point to it');
        $r = $this->request('GET', '/api/entities/toon:sandmouse?draft=1', $auth);
        self::assertSame(200, $r->getStatusCode(), 'its content index entry exists for the creator');
        self::assertSame(['sandmouse', 'draft', 'draft'], [self::json($r)['id'] ?? null, self::json($r)['status'] ?? null, $r->getHeaderLine('X-Content-Status')]);
        self::assertSame(422, $this->request('PUT', '/api/admin/catalog/toons/Sand', $auth, '{"data":{"id":"Sand","name":"S","redirectUrl":"https://scutoons.com/s"}}')->getStatusCode(), 'toon ids are content toon ids');
    }

    public function testTheFeaturedEndpoint(): void
    {
        $auth = $this->admin();
        $r = $this->request('PUT', '/api/admin/featured', $auth, '{"items":[{"product":"scorpion_amulet"}]}');
        self::assertSame([200, true], [$r->getStatusCode(), self::json($r)['changed']]);
        self::assertSame('[{"product":"scorpion_amulet"}]', (string) $this->request('GET', '/api/products/featured/')->getBody());
        $r = $this->request('PUT', '/api/admin/featured', $auth, '{"items":[{"product":"scorpion_amulet","variant":"gold"}]}');
        self::assertSame([400, 'unknown_variant'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(400, $this->request('PUT', '/api/admin/featured', $auth, '{"items":null}')->getStatusCode(), 'no default list to go back to');
        self::assertSame(404, $this->request('GET', '/api/admin/catalog/respawn', $auth)->getStatusCode(), 'respawn is edited in the item');
    }
}
