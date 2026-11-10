<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Live\LiveSessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** Show documents with versions and the `live` role (#90, research/SHOW.md §4b); live sessions (LIVE.md B.6). */
final class ShowsApiTest extends TestCase
{
    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + ['LIVE_TOKEN_SECRET' => ''] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $settings
     */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, array $settings = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $settings + $this->settings()));
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

    /**
     * @param list<array<string, mixed>> $seats
     * @return array<string, mixed>
     */
    private static function show(string $id, array $seats = []): array
    {
        return ['id' => $id, 'outlets' => [['id' => 'left']], 'seats' => $seats, 'rows' => []];
    }

    public function testShowsAreSavedWithVersionsAndTheLiveRoleDecides(): void
    {
        $live = $this->session([Roles::LIVE], 'live@example.test');
        $creator = $this->session([Roles::CREATOR], 'creator@example.test');

        self::assertSame(403, $this->request('PUT', '/api/shows/top', $creator, ['content' => self::show('top')])->getStatusCode(), 'creator does not imply live');
        self::assertSame(401, $this->request('GET', '/api/shows')->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/entities/show:top', $live, ['content' => self::show('top')])->getStatusCode(), 'only /api/shows takes a show');

        $r = $this->request('PUT', '/api/shows/top', $live, ['content' => self::show('top')]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $doc = self::json($this->request('GET', '/api/shows/top?draft=1', $live))['content'];
        self::assertSame(1, $doc['version']);

        // an unchanged document keeps its number, a changed one gets the next; both stay readable by the number a seat pins
        $this->request('PUT', '/api/shows/top', $live, ['content' => self::show('top')]);
        self::assertSame(1, self::json($this->request('GET', '/api/shows/top?draft=1', $live))['content']['version']);
        $changed = self::show('top');
        $changed['rows'] = [['id' => 'a']];
        $this->request('PUT', '/api/shows/top', $live, ['content' => $changed]);
        self::assertSame(2, self::json($this->request('GET', '/api/shows/top?draft=1', $live))['content']['version']);
        self::assertSame([], self::json($this->request('GET', '/api/shows/top?v=1', $live))['show']['rows']);
        self::assertSame(404, $this->request('GET', '/api/shows/top?v=9', $live)->getStatusCode());

        self::assertCount(2, self::json($this->request('GET', '/api/shows/top/versions', $live))['versions']);
        self::assertSame('show:top', self::json($this->request('GET', '/api/shows', $live))['shows'][0]['key']);

        // the document names itself
        self::assertSame(422, $this->request('PUT', '/api/shows/top', $live, ['content' => self::show('other')])->getStatusCode());
    }

    public function testPublishingAsksTheCoreWithTheWholeTreeByValue(): void
    {
        $live = $this->session([Roles::LIVE], 'tree@example.test');
        $this->request('PUT', '/api/shows/foyer', $live, ['content' => self::show('foyer')]);
        $seat = ['id' => 'foyer', 'show' => ['id' => 'foyer', 'version' => 1], 'word' => 'follow'];
        $this->request('PUT', '/api/shows/top', $live, ['content' => self::show('top', [$seat])]);

        $this->core->answer = fn(array $request): array => ['a problem'];
        $r = $this->request('POST', '/api/shows/top/publish', $live, []);
        self::assertSame(422, $r->getStatusCode());
        self::assertSame(['a problem'], self::json($r)['problems']);
        $asked = $this->core->requests[0]['request'];
        self::assertSame('shows', $asked['kind']);
        self::assertSame('foyer', $asked['shows'][0]['id']);

        $this->core->answer = fn(array $request): array => [];
        self::assertSame(200, $this->request('POST', '/api/shows/top/publish', $live, [])->getStatusCode());
    }

    public function testASessionIsACodeAndTheRelaysTokens(): void
    {
        $live = $this->session([Roles::LIVE], 'session@example.test');
        self::assertSame(403, $this->request('POST', '/api/live/sessions', $this->session([Roles::CREATOR], 'c2@example.test'), [])->getStatusCode());

        $r = $this->request('POST', '/api/live/sessions', $live, ['show' => 'top']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $open = self::json($r);
        self::assertMatchesRegularExpression('/^[A-Z2-9]{6}$/', $open['code']);
        self::assertSame('dev', $open['token'], 'no secret: the relay in dev mode takes any token');

        $signed = self::json($this->request('POST', '/api/live/sessions', $live, ['show' => 'top'], ['LIVE_TOKEN_SECRET' => 's3cret']));
        self::assertFalse($signed['dev']);
        [$payload] = explode('.', $signed['tokens']['seat']);
        $claims = json_decode((string) base64_decode(strtr($payload, '-_', '+/')), true);
        self::assertSame('seat', $claims['role']);
        self::assertSame($signed['code'], $claims['session']);
    }

    public function testTheTokenIsWhatTheRelayExpects(): void
    {
        // the same vector as deck/relay/src/token.rs `signs_like_the_api`
        self::assertSame(
            'eyJzZXNzaW9uIjoiQTdLMiIsInJvbGUiOiJjb25kdWN0b3IiLCJleHAiOjIwMDAsInNob3ciOiJ0b3AifQ.VKZMHFY4kDbr-uvH_Wiu-Pw07Cpc9UdvLkp6oc8B-6Y',
            LiveSessions::sign('shared-with-the-api', ['session' => 'A7K2', 'role' => 'conductor', 'exp' => 2000, 'show' => 'top'])
        );
    }
}
