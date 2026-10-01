<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Domain\RequestContext;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The Slim app in-process: middleware order, error shape, CORS, role guard, trusted proxy (qr app), versions */
final class AppTest extends TestCase
{
    protected function settings(): array
    {
        return [
            'ALLOWED_ORIGINS' => 'http://viewer.test,http://admin.test',
            'QR_PROXY_SECRET' => 'proxy-secret-for-tests',
            'VERSION_URLS' => 'viewer=http://127.0.0.1:1/version.json,bad=ftp://x',
        ] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], ?string $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, $uri, ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write($body);
            $request->getBody()->rewind();
        }
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    public function testErrorShapeAndUnknownEndpoints(): void
    {
        $r = $this->request('GET', 'http://localhost/api/nope/');
        self::assertSame(404, $r->getStatusCode());
        self::assertSame(['status' => 'error', 'error' => 'not_found', 'message' => 'Unknown endpoint.'], self::json($r));
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        self::assertSame(404, $this->request('POST', 'http://localhost/api/me/')->getStatusCode(), 'wrong method = unknown endpoint');
        $r = $this->request('POST', 'http://localhost/api/auth/request/', [], '{bad');
        self::assertSame([400, 'invalid_json'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame(401, $this->request('GET', 'http://localhost/api/me')->getStatusCode(), 'trailing slash optional');
    }

    public function testCorsForAllowedOriginsOnly(): void
    {
        $r = $this->request('OPTIONS', 'http://localhost/api/me/', ['Origin' => 'http://viewer.test']);
        self::assertSame([204, 'http://viewer.test'], [$r->getStatusCode(), $r->getHeaderLine('Access-Control-Allow-Origin')]);
        self::assertStringContainsString('X-Device-Id', $r->getHeaderLine('Access-Control-Allow-Headers'));
        self::assertStringNotContainsString('DELETE', $r->getHeaderLine('Access-Control-Allow-Methods'));
        self::assertStringContainsString('DELETE', $this->request('OPTIONS', 'http://localhost/api/admin/roles', ['Origin' => 'http://admin.test'])->getHeaderLine('Access-Control-Allow-Methods'));
        self::assertFalse($this->request('OPTIONS', 'http://localhost/api/me/', ['Origin' => 'https://evil.test'])->hasHeader('Access-Control-Allow-Origin'));
        self::assertSame('http://viewer.test', $this->request('GET', 'http://localhost/api/me/', ['Origin' => 'http://viewer.test'])->getHeaderLine('Access-Control-Allow-Origin'), 'errors carry CORS too');
        self::assertSame('*', $this->request('GET', 'http://localhost/api/items/', ['Origin' => 'https://evil.test'])->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertSame('*', $this->request('OPTIONS', 'http://localhost/api/validate/', ['Origin' => 'https://evil.test'])->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertFalse($this->request('GET', 'http://localhost/api/toons/', ['Origin' => 'http://viewer.test'])->hasHeader('Access-Control-Allow-Origin'));
    }

    public function testProductsAreTheStoredFilesByteForByte(): void
    {
        $root = dirname(__DIR__, 3);
        foreach (['products' => 'products.json', 'products/featured' => 'featured-products.json'] as $path => $file) {
            $r = $this->request('GET', "http://localhost/api/$path/", ['Origin' => 'https://evil.test']);
            self::assertSame(200, $r->getStatusCode(), $path);
            self::assertSame((string) file_get_contents("$root/config/$file"), (string) $r->getBody(), $path);
            self::assertSame('application/json; charset=utf-8', $r->getHeaderLine('Content-Type'));
            self::assertSame(['*', 'public, max-age=300'], [$r->getHeaderLine('Access-Control-Allow-Origin'), $r->getHeaderLine('Cache-Control')]);
        }
        self::assertSame('http://viewer.test', $this->request('GET', 'http://localhost/api/products', ['Origin' => 'http://viewer.test'])->getHeaderLine('Access-Control-Allow-Origin'));
        $r = $this->request('PUT', 'http://localhost/api/products/featured/');
        self::assertSame([405, 'method_not_allowed'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testAdminRoutesNeedTheAdminRole(): void
    {
        $user = $this->user();
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        self::assertSame(401, $this->request('GET', 'http://localhost/api/admin/roles')->getStatusCode());
        $r = $this->request('GET', 'http://localhost/api/admin/roles', ['Authorization' => "Bearer $token"]);
        self::assertSame([403, 'forbidden', 'admin'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['role']]);
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        $r = $this->request('GET', 'http://localhost/api/admin/roles', ['Authorization' => "Bearer $token"]);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['member', 'admin', 'creator', 'liaison', 'ambassador'], array_column(self::json($r)['items'], 'name'));
    }


    public function testApiOnlyNoPublicPages(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $r = $this->request('GET', "http://localhost/?c=$code");
        self::assertSame([404, 'Not found'], [$r->getStatusCode(), (string) $r->getBody()], 'the landing page lives in the qr app');
        self::assertSame(404, $this->request('GET', 'http://localhost/verify.html')->getStatusCode());
        self::assertSame(200, $this->request('GET', "http://api.test/api/validate/?c=$code")->getStatusCode(), 'proxy target of the qr app');
        self::assertSame(401, $this->request('GET', 'http://qr.test/api/me/')->getStatusCode(), 'no host routing any more');
        $r = $this->request('GET', 'http://localhost/api/health/');
        self::assertSame(['ok', 'ok', '1.0.0', 'dev'], [self::json($r)['status'], self::json($r)['db'], self::json($r)['version'], self::json($r)['git_sha']]);
    }

    public function testForwardedClientOnlyWithTheProxySecret(): void
    {
        $code = $this->code('membership', ['redirect_to' => 'https://scutoons.com/catpope']);
        $context = $this->get(RequestContext::class);
        $forwarded = ['X-Scutoons-Client-Ip' => '198.51.100.7', 'X-Scutoons-Device-Id' => 'device-from-proxy-1'];

        // Without / with a wrong secret: the forwarded headers are ignored, and so is scan=1
        $this->request('GET', "http://localhost/api/code/?c=$code&scan=1", $forwarded);
        self::assertSame(['127.0.0.1', null, false], [$context->clientIp(), $context->deviceId(), $context->viaTrustedProxy()]);
        $this->request('GET', "http://localhost/api/code/?c=$code&scan=1", $forwarded + ['X-Scutoons-Proxy-Secret' => 'wrong']);
        self::assertSame(['127.0.0.1', false, 0], [$context->clientIp(), $context->viaTrustedProxy(), $this->scans($code)]);

        $r = $this->request('GET', "http://localhost/api/code/?c=$code&scan=1", $forwarded + ['X-Scutoons-Proxy-Secret' => 'proxy-secret-for-tests']);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['198.51.100.7', 'device-from-proxy-1', true], [$context->clientIp(), $context->deviceId(), $context->viaTrustedProxy()]);
        self::assertSame(1, $this->scans($code));
        self::assertSame('catpope', $this->db->value("SELECT toon FROM code_events WHERE code = ? AND event = 'scan'", [$code]));

        // Expired / revoked codes are scanned too (410 afterwards), unknown codes are not
        $revoked = $this->code('membership', ['revoked_at' => $this->clock->local()]);
        $r = $this->request('GET', "http://localhost/api/code/?c=$revoked&scan=1", ['X-Scutoons-Proxy-Secret' => 'proxy-secret-for-tests']);
        self::assertSame([410, 'revoked', 1], [$r->getStatusCode(), self::json($r)['error'], $this->scans($revoked)]);
        self::assertSame(404, $this->request('GET', 'http://localhost/api/code/?c=zzzzzzzz&scan=1', ['X-Scutoons-Proxy-Secret' => 'proxy-secret-for-tests'])->getStatusCode());

        // A forwarded value that is not an IP falls back to REMOTE_ADDR
        $this->request('GET', "http://localhost/api/code/?c=$code", ['X-Scutoons-Proxy-Secret' => 'proxy-secret-for-tests', 'X-Scutoons-Client-Ip' => 'not-an-ip']);
        self::assertSame('127.0.0.1', $context->clientIp());
    }

    public function testVersionsOfTheDeployedApps(): void
    {
        $user = $this->user();
        $this->get(Roles::class)->grant((string) $user['id'], Roles::ADMIN);
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        $r =$this->request('GET', 'http://localhost/api/admin/versions', ['Authorization' => "Bearer $token"]);
        $body = self::json($r);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['api', '1.0.0'], [$body['api']['app'], $body['api']['version']]);
        self::assertCount(1, $body['apps'], 'only http(s) URLs');
        self::assertSame(['viewer', false], [$body['apps'][0]['app'], $body['apps'][0]['ok']]);
        self::assertSame(401, $this->request('GET', 'http://localhost/api/admin/versions')->getStatusCode());
    }

    private function scans(string $code): int
    {
        return (int) $this->db->value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'scan'", [$code]);
    }
}