<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
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
 * Deck scenes `scene:<id>` (#243, research/SCENES.md §3): drafts with stamped versions by the content editors, the core's
 * check before a publish, published scenes public. The fixture is the first real scene, `scene:nave` (deck-space
 * scene_document.rs writes it and checks it round-trips).
 */
final class ScenesApiTest extends TestCase
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
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body, JSON_PRESERVE_ZERO_FRACTION));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
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

    /**
     * @param list<string> $roles
     * @return array<string, string>
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @return array<string, mixed> the first real scene as stored (no version: the server stamps it) */
    private static function nave(): array
    {
        return (array) json_decode((string) file_get_contents(self::root() . '/tests/fixtures/scenes/nave.json'), true);
    }

    public function testCreatorsSaveDraftsWithVersionsAndOnlyThePublishedOneIsPublic(): void
    {
        $creator = $this->session([Roles::CREATOR], 'creator@example.test');
        $reader = $this->session([], 'reader@example.test');

        self::assertSame(401, $this->request('PUT', '/api/scenes/nave', [], ['content' => self::nave()])->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/scenes/nave', $reader, ['content' => self::nave()])->getStatusCode(), 'a reader saves nothing');
        $wrong = $this->request('PUT', '/api/entities/scene:nave', $creator, ['content' => self::nave()]);
        self::assertSame([400, 'wrong_route'], [$wrong->getStatusCode(), self::json($wrong)['error']], 'only /api/scenes stamps and checks a scene');

        $r = $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $draft = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame(1, $draft['version']);
        self::assertSame('Nave', $draft['stage']['name']);

        // an unchanged document keeps its number, a changed one gets the next
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        self::assertSame(1, self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content']['version']);
        $changed = self::nave();
        $changed['stage']['name'] = 'The nave';
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $changed]);
        self::assertSame(2, self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content']['version']);
        self::assertSame('Nave', self::json($this->request('GET', '/api/scenes/nave?v=1', $creator))['scene']['stage']['name']);
        self::assertCount(2, self::json($this->request('GET', '/api/scenes/nave/versions', $creator))['versions'], 'an unchanged save stores nothing');

        // nothing published: a draft never leaves the editor
        self::assertSame(404, $this->request('GET', '/api/scenes/nave')->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/scenes/nave?v=1')->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/scenes/nave?draft=1')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/scenes/nave/versions', $reader)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', '/api/scenes'))['scenes']);
        self::assertSame('scene:nave', self::json($this->request('GET', '/api/scenes', $creator))['scenes'][0]['key']);

        $first = self::json($this->request('GET', '/api/scenes/nave/versions', $creator))['versions'][1]['id'];
        self::assertSame(200, $this->request('POST', '/api/scenes/nave/publish', $creator, ['version' => $first])->getStatusCode());
        $public = self::json($this->request('GET', '/api/scenes/nave'));
        self::assertSame(['nave', 1, 'Nave'], [$public['scene']['id'], $public['scene']['version'], $public['scene']['stage']['name']]);
        self::assertSame(1, self::json($this->request('GET', '/api/scenes/nave?v=1'))['scene']['version']);
        self::assertSame(404, $this->request('GET', '/api/scenes/nave?v=2')->getStatusCode(), 'v2 is a draft');
        $listed = self::json($this->request('GET', '/api/scenes', $reader))['scenes'];
        self::assertSame([['key' => 'scene:nave', 'content' => $public['scene']]], $listed, 'readers see the published document, not who saved it');

        // restore: the old content as the newest draft, stamped anew (it differs from v2: v3)
        $restored = $this->request('POST', '/api/scenes/nave/restore', $creator, ['id' => $first]);
        self::assertSame(200, $restored->getStatusCode(), (string) $restored->getBody());
        $draft = self::json($this->request('GET', '/api/scenes/nave?draft=1', $creator))['content'];
        self::assertSame([3, 'Nave'], [$draft['version'], $draft['stage']['name']]);
    }

    public function testTheShapeIsCheckedOnSave(): void
    {
        $creator = $this->session([Roles::CREATOR], 'shape@example.test');
        $other = self::nave();
        $other['id'] = 'other';
        self::assertSame(422, $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $other])->getStatusCode(), 'the document names itself');
        foreach ([
            'format' => fn(array $d) => ['format' => 2] + $d,
            'an unknown field' => fn(array $d) => ['nodes' => []] + $d,
            // SCENES.md §8.2: a nudge's control input is `input`; `set` in a scene is the staging alone
            'a nudge that says set' => function (array $d): array {
                $d['being']['react'] = [['mood' => 'happy', 'set' => 'space.hue', 'by' => 0.1]];
                return $d;
            },
        ] as $what => $change) {
            $r = $this->request('PUT', '/api/scenes/nave', $creator, ['content' => $change(self::nave())]);
            self::assertSame(422, $r->getStatusCode(), "$what: " . $r->getBody());
            self::assertSame('invalid_content', self::json($r)['error'], $what);
        }
        self::assertSame([], $this->core->requests, 'a save never asks the core');
    }

    public function testPublishingAsksTheCoreAndRefusesWhatADeckCouldNotPlay(): void
    {
        $creator = $this->session([Roles::CREATOR], 'publish@example.test');
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);

        $this->core->answer = fn(array $request): array => ["node 'a': unknown kind 'blender'"];
        $r = $this->request('POST', '/api/scenes/nave/publish', $creator, []);
        self::assertSame(422, $r->getStatusCode());
        self::assertSame('invalid_scene', self::json($r)['error']);
        self::assertSame(["node 'a': unknown kind 'blender'"], self::json($r)['problems']);
        $asked = $this->core->requests[0]['request'];
        self::assertSame(['scenes', 'nave', 'nave'], [$asked['kind'], $asked['id'], $asked['document']['id']]);
        self::assertSame(404, $this->request('GET', '/api/scenes/nave')->getStatusCode(), 'nothing published');

        $this->core->answer = fn(array $request): array => [];
        self::assertSame(200, $this->request('POST', '/api/scenes/nave/publish', $creator, [])->getStatusCode());
        self::assertSame(200, $this->request('GET', '/api/scenes/nave')->getStatusCode());
    }

    public function testWithoutTheBuildServiceNothingIsPublished(): void
    {
        $creator = $this->session([Roles::CREATOR], 'nobuild@example.test');
        $this->request('PUT', '/api/scenes/nave', $creator, ['content' => self::nave()]);
        $this->core->answer = fn(array $request): \RuntimeException => new \RuntimeException('down');
        self::assertSame(503, $this->request('POST', '/api/scenes/nave/publish', $creator, [])->getStatusCode());
    }

    public function testAnAiConnectionSavesDraftsButNeverPublishes(): void
    {
        $user = $this->user('ai@example.test');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $token = $this->get(ApiTokens::class)->create($user, 'Claude')['token'];
        $ai = ['Authorization' => "Bearer $token"];
        $r = $this->request('PUT', '/api/scenes/nave', $ai, ['content' => self::nave(), 'message' => 'first']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('first (AI: Claude)', self::json($r)['version']['message']);
        self::assertSame(403, $this->request('POST', '/api/scenes/nave/publish', $ai, [])->getStatusCode());
    }
}
