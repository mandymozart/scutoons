<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/embassy/*: role gate, CORS for the Embassy origin, a full round trip (create → call → today → import) */
final class EmbassyApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['ALLOWED_ORIGINS' => 'http://embassy.test'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
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
    private function auth(string $role): array
    {
        $user = $this->user();
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    public function testRoleGate(): void
    {
        self::assertSame(401, $this->request('GET', '/api/embassy/me')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/embassy/me', $this->auth(''))->getStatusCode());
        $r = $this->request('GET', '/api/embassy/me/', $this->auth('liaison'));
        self::assertSame([200, 'liaison', ['live', 'vinyl', 'booking', 'promo']], [$r->getStatusCode(), self::json($r)['access'], self::json($r)['tracks']]);
        self::assertSame('ambassador', self::json($this->request('GET', '/api/embassy/me', $this->auth('ambassador')))['access']);
    }

    public function testCorsAllowsDeleteForTheEmbassyOrigin(): void
    {
        $r = $this->request('OPTIONS', '/api/embassy/orgs', ['Origin' => 'http://embassy.test']);
        self::assertSame([204, 'http://embassy.test', 'GET, POST, PUT, DELETE, OPTIONS'],
            [$r->getStatusCode(), $r->getHeaderLine('Access-Control-Allow-Origin'), $r->getHeaderLine('Access-Control-Allow-Methods')]);
    }

    public function testRoundTrip(): void
    {
        $liaison = $this->auth('liaison');
        $r = $this->request('POST', '/api/embassy/orgs', $liaison, ['name' => 'Dachstock', 'type' => 'venue', 'city' => 'Bern', 'country' => 'CH',
            'people' => [['name' => 'Kathi', 'email' => 'booking@dachstock.ch', 'phone' => '+41 31 000']], 'leads' => [['track' => 'live', 'next_at' => '2026-09-21']]]);
        self::assertSame(201, $r->getStatusCode());
        $org = self::json($r);
        $id = $org['org']['id'];

        $today = self::json($this->request('GET', '/api/embassy/today', $liaison));
        self::assertSame(['Dachstock', 'Kathi'], [$today['due'][0]['org']['name'], $today['due'][0]['people'][0]['name']]);

        $r = $this->request('POST', "/api/embassy/orgs/$id/activities", $liaison, ['kind' => 'call', 'track' => 'live', 'outcome' => 'voicemail', 'person_id' => $org['people'][0]['id']]);
        self::assertSame([201, '2026-09-24'], [$r->getStatusCode(), self::json($r)['lead']['next_at']]);
        self::assertSame([], self::json($this->request('GET', '/api/embassy/today', $liaison))['due']);

        $r = $this->request('PUT', "/api/embassy/orgs/$id/leads/live", $liaison, ['stage' => 'bogus']);
        self::assertSame([400, 'invalid_stage'], [$r->getStatusCode(), self::json($r)['error']]);
        $r = $this->request('PUT', "/api/embassy/orgs/$id", $liaison, ['tier' => 'A', 'notes' => 'Kathi books Thursdays']);
        self::assertSame('A', self::json($r)['org']['tier']);
        self::assertSame(1, self::json($this->request('GET', '/api/embassy/orgs?tier=A', $liaison))['total']);

        self::assertSame(403, $this->request('DELETE', "/api/embassy/orgs/$id", $liaison)->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/embassy/import', $liaison, ['organisations' => [['name' => 'X']]])->getStatusCode());

        $amb = $this->auth('ambassador');
        $r = $this->request('POST', '/api/embassy/import', $amb, ['organisations' => [['name' => 'Dachstock', 'city' => 'Bern', 'tags' => ['punk']], ['name' => 'Openground', 'city' => 'Wuppertal']]]);
        self::assertSame([200, 1, 1], [$r->getStatusCode(), self::json($r)['created'], self::json($r)['merged']]);
        self::assertCount(2, self::json($this->request('GET', '/api/embassy/team', $amb))['items']);
        self::assertSame(200, $this->request('DELETE', "/api/embassy/orgs/$id", $amb)->getStatusCode());
        self::assertSame(404, $this->request('GET', "/api/embassy/orgs/$id", $amb)->getStatusCode());
    }
}
