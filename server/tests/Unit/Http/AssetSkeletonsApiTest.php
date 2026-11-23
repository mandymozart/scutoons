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
 * Shared animations (#817): a .glb in an `animations/` folder is an asset of kind `animation`; a model's or an
 * animation's skeleton through `PUT / GET /api/assets/{id}` and the list (the core decides, `/validate` kind
 * `skeleton`); a character is saved only with idles that play on its models (kind `animation-fit`). The requests and the
 * core's answers are the shared vectors server/tests/fixtures/assets/skeletons.json (deck-space checks the answers).
 */
final class AssetSkeletonsApiTest extends TestCase
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

    /** @return array<string, string> */
    private function creator(): array
    {
        $user = $this->user('creator@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], 'creator');
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @param array<string, string> $creator */
    private function upload(array $creator, string $path): string
    {
        $up = $this->request('PUT', "/api/assets/upload?path=$path", $creator + ['Content-Type' => 'model/gltf-binary'], "glTF\x02\0\0\0\x0c\0\0\0");
        self::assertSame(201, $up->getStatusCode(), (string) $up->getBody());
        return (string) self::json($up)['asset']['id'];
    }

    /** @return list<array<string, mixed>> */
    private static function cases(): array
    {
        /** @var array{cases: list<array<string, mixed>>} $set */
        $set = json_decode((string) file_get_contents(self::root() . '/tests/fixtures/assets/skeletons.json'), true);
        return $set['cases'];
    }

    public function testAnAnimationIsAKindAndItsSkeletonIsTheCoresDocument(): void
    {
        $creator = $this->creator();
        $json = $creator + ['Content-Type' => 'application/json'];
        $id = $this->upload($creator, 'characters/animations/catpope-idle.glb');
        self::assertSame('animation', self::json($this->request('GET', "/api/assets/$id", $creator))['asset']['kind']);
        self::assertSame('model', self::json($this->request('GET', '/api/assets/' . $this->upload($creator, 'characters/crow-fox/crow-fox.glb'), $creator))['asset']['kind']);

        foreach (self::cases() as $case) {
            if ($case['request']['kind'] !== 'skeleton') continue;
            $asked = count($this->core->requests);
            $this->core->answer = fn(array $request): array => $case['problems'];
            $set = $this->request('PUT', "/api/assets/$id", $json, (string) json_encode(['skeleton' => $case['request']['document']]));
            self::assertCount($asked + 1, $this->core->requests, "{$case['name']}: one question to the core");
            self::assertEquals(['id' => $id] + $case['request'], $this->core->requests[$asked]['request'], "{$case['name']}: the request of the shared vectors");
            if ($case['problems'] === []) {
                self::assertSame(200, $set->getStatusCode(), $case['name'] . ': ' . $set->getBody());
                self::assertEquals($case['request']['document'], self::json($set)['versions'][0]['skeleton'], "{$case['name']}: as sent");
            } else {
                self::assertSame(422, $set->getStatusCode(), $case['name']);
                self::assertSame(['invalid_skeleton', $case['problems']], [self::json($set)['error'], self::json($set)['problems']], "{$case['name']}: the core's words");
            }
        }
        $this->core->answer = fn(array $request): array => [];
        $this->request('PUT', "/api/assets/$id", $json, (string) json_encode(['skeleton' => self::cases()[0]['request']['document']]));
        $list = self::json($this->request('GET', '/api/assets?kind=animation', $creator));
        self::assertSame([$id], array_column($list['items'], 'id'), 'the library filters animations');
        self::assertSame('mixamo', $list['items'][0]['skeleton']['family'] ?? null, 'the list carries the skeleton (the picker filters by it)');

        self::assertSame(422, $this->request('PUT', "/api/assets/$id", $json, '{"skeleton":"mixamo"}')->getStatusCode());
        $image = $this->request('PUT', '/api/assets/upload?path=characters/x.png', $creator + ['Content-Type' => 'image/png'], (string) base64_decode('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=='));
        self::assertSame(422, $this->request('PUT', '/api/assets/' . self::json($image)['asset']['id'], $json, '{"skeleton":{"joints":[]}}')->getStatusCode(), 'an image has no skeleton');
    }

    public function testACharacterIsSavedOnlyWithAnIdleThatPlaysOnItsModel(): void
    {
        $creator = $this->creator();
        $json = $creator + ['Content-Type' => 'application/json'];
        $anim = $this->upload($creator, 'characters/animations/catpope-idle.glb');
        $model = $this->upload($creator, 'characters/crow-fox/crow-fox.glb');
        $fits = array_values(array_filter(self::cases(), fn(array $c) => $c['name'] === 'an idle that fits'))[0]['request']['document'];
        $character = fn(array $idle): string => (string) json_encode(['content' => ['name' => ['en' => 'Crow Fox'], 'color' => '#aa5500', 'personas' => [['id' => 'regular', 'model' => 'characters/crow-fox/crow-fox.glb', 'animations' => ['idle' => $idle],
            'looks' => [['id' => 'default', 'default' => true, 'images' => new \stdClass()]]]]]]);
        $save = fn(array $idle) => $this->request('PUT', '/api/entities/character:crow-fox', $json, $character($idle));

        $unread = $save(['animation' => 'characters/animations/catpope-idle.glb']);
        self::assertSame(422, $unread->getStatusCode(), (string) $unread->getBody());
        self::assertStringContainsString('has no skeleton read yet', implode(' ', self::json($unread)['problems']));

        $this->core->answer = fn(array $request): array => [];
        $this->request('PUT', "/api/assets/$anim", $json, (string) json_encode(['skeleton' => $fits['animation']]));
        $this->request('PUT', "/api/assets/$model", $json, (string) json_encode(['skeleton' => $fits['model']]));
        $this->core->requests = [];
        self::assertSame(200, $save(['animation' => 'characters/animations/catpope-idle.glb'])->getStatusCode());
        self::assertEquals(['kind' => 'animation-fit', 'id' => 'character:crow-fox/regular', 'document' => $fits], $this->core->requests[0]['request'], 'the core is asked with both skeletons');

        $this->core->answer = fn(array $request): array => ["animation of character:crow-fox/regular: no clip 'run': the model has walk, stand"];
        $refused = $save(['clip' => 'run']);
        self::assertSame(422, $refused->getStatusCode());
        self::assertSame(["animation of character:crow-fox/regular: no clip 'run': the model has walk, stand"], self::json($refused)['problems'], "the core's words");
        self::assertSame('run', $this->core->requests[count($this->core->requests) - 1]['request']['document']['clip']);

        $this->core->requests = [];
        self::assertSame(200, $this->request('PUT', '/api/entities/character:crow-fox', $json, (string) json_encode(['content' => ['name' => ['en' => 'Crow Fox'], 'color' => '#aa5500', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => new \stdClass()]]]]]]))->getStatusCode());
        self::assertSame([], $this->core->requests, 'no idle: the core is not asked');
    }
}
