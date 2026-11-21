<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * A model version's spots through `PUT / GET /api/assets/{id}` (#724): the core decides (the build service's `/validate`
 * kind `spots`), the API stores what it was sent. The requests and the core's answers are the shared vectors
 * server/tests/fixtures/assets/spots.json (deck-space checks the answers, deck-build the route).
 */
final class AssetSpotsApiTest extends TestCase
{
    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + ['ASSET_STORAGE' => 'local', 'ASSET_LOCAL_ROOT' => $this->dir . '/cdn', 'CDN_URL' => 'http://cdn.test/'] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
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

    public function testSpotsRoundTripOnTheCurrentVersionAndTheCoreDecides(): void
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], 'creator');
        $creator = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
        $json = $creator + ['Content-Type' => 'application/json'];
        $glb = "glTF\x02\0\0\0\x0c\0\0\0";
        $up = $this->request('PUT', '/api/assets/upload?path=scutoons/3d/lantern.glb', $creator + ['Content-Type' => 'model/gltf-binary'], $glb);
        self::assertSame(201, $up->getStatusCode(), (string) $up->getBody());
        $id = (string) self::json($up)['asset']['id'];
        self::assertArrayNotHasKey('spots', self::json($this->request('GET', "/api/assets/$id", $creator))['versions'][0], 'none stored: none carried');

        $cases = (array) json_decode((string) file_get_contents(self::root() . '/tests/fixtures/assets/spots.json'), true)['cases'];
        foreach ($cases as $case) {
            if (isset($case['request']['modules'])) continue; // a module's body: no asset names its module yet
            $asked = count($this->core->requests);
            $this->core->answer = fn(array $request): array => $case['problems'];
            $spots = $case['request']['document'];
            $set = $this->request('PUT', "/api/assets/$id", $json, (string) json_encode(['spots' => $spots], JSON_PRESERVE_ZERO_FRACTION));
            self::assertCount($asked + 1, $this->core->requests, "{$case['name']}: one question to the core");
            $sent = $this->core->requests[$asked];
            self::assertSame('POST ' . FakeCore::URL . '/validate', $sent['url']);
            self::assertSame(FakeCore::TOKEN, $sent['headers']['X-Build-Token']);
            self::assertEquals(['id' => $id] + $case['request'], $sent['request'], "{$case['name']}: the request of the shared vectors");
            if ($case['problems'] === []) {
                self::assertSame(200, $set->getStatusCode(), $case['name'] . ': ' . $set->getBody());
                self::assertEquals($spots, self::json($set)['versions'][0]['spots'], "{$case['name']}: as sent");
                self::assertEquals($spots, self::json($this->request('GET', "/api/assets/$id", $creator))['versions'][0]['spots']);
            } else {
                self::assertSame(422, $set->getStatusCode(), $case['name']);
                self::assertSame(['invalid_spots', $case['problems']], [self::json($set)['error'], self::json($set)['problems']], "{$case['name']}: the core's words, unchanged");
            }
        }
        $kept = self::json($this->request('GET', "/api/assets/$id", $creator))['versions'][0]['spots'];
        self::assertEquals($cases[0]['request']['document'], $kept, 'a refused save changes nothing');

        // only the shape is the API's: an object or null; null (or nothing in it) clears without asking the core
        $this->core->requests = [];
        foreach (['{"spots":"hand"}', '{"spots":[1]}'] as $bad) {
            self::assertSame(422, $this->request('PUT', "/api/assets/$id", $json, $bad)->getStatusCode(), $bad);
        }
        $none = $this->request('PUT', "/api/assets/$id", $json, '{"spots":null}');
        self::assertSame(200, $none->getStatusCode());
        self::assertArrayNotHasKey('spots', self::json($none)['versions'][0]);
        self::assertSame([], $this->core->requests);

        // a title alone never touches the spots; without the build service nothing is stored – never a silent pass
        $this->core->answer = fn(array $request): array => [];
        $this->request('PUT', "/api/assets/$id", $json, (string) json_encode(['spots' => $cases[0]['request']['document']]));
        $titled = self::json($this->request('PUT', "/api/assets/$id", $json, '{"title":"Lantern"}'));
        self::assertSame('Lantern', $titled['asset']['title']);
        self::assertArrayHasKey('spots', $titled['versions'][0]);
        $this->core->answer = fn(array $request): \RuntimeException => new \RuntimeException('down');
        self::assertSame(503, $this->request('PUT', "/api/assets/$id", $json, '{"spots":{"hand":[{"name":"x"}]}}')->getStatusCode());
        self::assertSame(403, $this->request('PUT', "/api/assets/$id", ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user('member@example.com')['id']), 'Content-Type' => 'application/json'], '{"spots":null}')->getStatusCode());
    }
}
