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

/** Admin API /api/admin/settings and the rate limits it controls (lookups per IP, QR scans through the qr app) */
final class SettingsApiTest extends TestCase
{
    private const SECRET = 'proxy-secret-for-tests';

    protected function settings(): array
    {
        return ['QR_PROXY_SECRET' => self::SECRET] + parent::settings();
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
        // One container per request, as in production (the settings are read once per request)
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    private function adminToken(): string
    {
        $user = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        return $this->get(Sessions::class)->create((string) $user['id']);
    }

    private function scan(string $code, string $ip): int
    {
        return $this->request('GET', "/api/code/?c=$code&scan=1", ['X-Scutoons-Proxy-Secret' => self::SECRET, 'X-Scutoons-Client-Ip' => $ip])->getStatusCode();
    }

    public function testReadAndUpdateNeedTheAdminRole(): void
    {
        self::assertSame(401, $this->request('GET', '/api/admin/settings')->getStatusCode());
        $member = $this->get(Sessions::class)->create((string) $this->user()['id']);
        self::assertSame(403, $this->request('PUT', '/api/admin/settings', ['Authorization' => "Bearer $member"], '{"scan_limit_per_hour":1}')->getStatusCode());

        $auth = ['Authorization' => 'Bearer ' . $this->adminToken()];
        $r = $this->request('GET', '/api/admin/settings', $auth);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame([1000, 120, 200], array_column(self::json($r)['items'], 'value'));

        $r = $this->request('PUT', '/api/admin/settings', $auth, '{"scan_limit_per_hour":3}');
        $body = self::json($r);
        self::assertSame([200, ['scan_limit_per_hour'], 3, 'admin:boss@example.com'], [$r->getStatusCode(), $body['changed'], $body['items'][0]['value'], $body['items'][0]['updated_by']]);
        $r = $this->request('PUT', '/api/admin/settings', $auth, '{"settings":{"lookup_limit_per_hour":-5}}');
        self::assertSame([400, 'invalid_setting', 'lookup_limit_per_hour'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['key']]);
        $r = $this->request('PUT', '/api/admin/settings', $auth, '{"bogus":1}');
        self::assertSame([400, 'unknown_setting'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testScansHaveTheirOwnLimitPerForwardedIp(): void
    {
        $code = $this->code('membership');
        $auth = ['Authorization' => 'Bearer ' . $this->adminToken()];
        $this->request('PUT', '/api/admin/settings', $auth, '{"scan_limit_per_hour":3,"lookup_limit_per_hour":2}');

        self::assertSame([200, 200, 200, 429], [$this->scan($code, '203.0.113.1'), $this->scan($code, '203.0.113.1'), $this->scan($code, '203.0.113.1'), $this->scan($code, '203.0.113.1')]);
        self::assertSame(200, $this->scan($code, '203.0.113.2'), 'another client IP has its own bucket');
        // Direct lookups (no scan / no proxy) count in the lookup bucket, not the scan bucket
        self::assertSame([200, 200, 429], [
            $this->request('GET', "/api/code/?c=$code")->getStatusCode(),
            $this->request('GET', "/api/access/?c=$code")->getStatusCode(),
            $this->request('GET', "/api/code/?c=$code&scan=1")->getStatusCode(), // scan=1 without the proxy secret = a lookup
        ]);
        $r = $this->request('GET', "/api/code/?c=$code");
        self::assertSame([429, 'rate_limited', 3600], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['retry_after']]);

        // 0 = no limit (nothing counted)
        $this->request('PUT', '/api/admin/settings', $auth, '{"scan_limit_per_hour":0,"lookup_limit_per_hour":0}');
        self::assertSame([200, 200], [$this->scan($code, '203.0.113.1'), $this->request('GET', "/api/code/?c=$code")->getStatusCode()]);
    }
}
