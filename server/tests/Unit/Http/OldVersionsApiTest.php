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

/** GET /api/admin/content/old-versions + POST …/purge (issue 154): admin only, dry run, then the delete with its counts */
final class OldVersionsApiTest extends TestCase
{
    /**
     * @param array<string, string> $headers
     * @param array<string, mixed>|null $body
     */
    private function request(string $method, string $uri, array $headers = [], ?array $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write((string) json_encode($body));
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

    private function version(string $key, string $createdAt, string $status = 'draft'): void
    {
        $this->db->run('INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES (?, ?, NULL, NULL, ?, NULL, ?, ?, NULL)',
            [$key, explode(':', $key)[0], '{}', $createdAt, $status]);
    }

    public function testOnlyAdminsSeeOrPurge(): void
    {
        $member = ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $this->user()['id'])];
        self::assertSame(401, $this->request('GET', '/api/admin/content/old-versions?before=2026-08-01')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/admin/content/old-versions?before=2026-08-01', $member)->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/admin/content/old-versions/purge', $member, ['before' => '2026-08-01', 'documents' => 0, 'versions' => 0])->getStatusCode());
    }

    public function testDryRunThenPurge(): void
    {
        $auth = $this->admin();
        $this->version('characters', '2026-06-01 00:00:00', 'published');
        $this->version('characters', '2026-06-02 00:00:00');
        $this->version('characters', '2026-06-03 00:00:00');
        $this->version('characters', '2026-09-01 00:00:00');

        $plan = $this->request('GET', '/api/admin/content/old-versions?before=2026-08-01', $auth);
        self::assertSame(200, $plan->getStatusCode());
        $plan = self::json($plan);
        self::assertSame(['success', 1, 2, 'characters'], [$plan['status'], $plan['documents'], $plan['versions'], $plan['items'][0]['key']]);

        $stale = $this->request('POST', '/api/admin/content/old-versions/purge', $auth, ['before' => '2026-08-01', 'documents' => 1, 'versions' => 5]);
        self::assertSame(409, $stale->getStatusCode());
        self::assertSame(['plan_changed', 2], [self::json($stale)['error'], self::json($stale)['versions']]);

        $done = $this->request('POST', '/api/admin/content/old-versions/purge', $auth, ['before' => '2026-08-01', 'documents' => 1, 'versions' => 2]);
        self::assertSame(200, $done->getStatusCode());
        self::assertSame(2, self::json($done)['deleted']);
        self::assertSame(2, (int) $this->db->value("SELECT COUNT(*) FROM entity_versions WHERE entity_key = 'characters'"));

        self::assertSame(422, $this->request('GET', '/api/admin/content/old-versions?before=tomorrow', $auth)->getStatusCode());
    }
}
