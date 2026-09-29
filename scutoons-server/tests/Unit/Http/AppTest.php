<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\Middleware\HostRouting;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** The Slim app in-process: middleware order, error shape, CORS, host routing, role guard */
final class AppTest extends TestCase
{
    protected function settings(): array
    {
        return ['QR_HOST' => 'qr.test', 'API_HOST' => 'api.test', 'ALLOWED_ORIGINS' => 'http://viewer.test,http://admin.test'] + parent::settings();
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
        self::assertSame(['member', 'admin', 'creator'], array_column(self::json($r)['items'], 'name'));
    }

    public function testHostRouting(): void
    {
        $code = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        self::assertSame(200, $this->request('GET', "http://qr.test/?c=$code")->getStatusCode());
        self::assertSame(200, $this->request('GET', "http://qr.test/api/validate/?c=$code")->getStatusCode());
        self::assertSame(401, $this->request('GET', 'http://qr.test/api/me/')->getStatusCode(), 'transition: full API on the qr host');
        self::assertSame(404, $this->request('GET', "http://api.test/?c=$code")->getStatusCode(), 'no landing page on the api host');
        self::assertSame(200, $this->request('GET', "http://api.test/api/validate/?c=$code")->getStatusCode());
        self::assertSame(200, $this->request('GET', 'http://api.test/api/health/')->getStatusCode());
        self::assertSame(HostRouting::PUBLIC_API, HostRouting::area('/api/keys/keys.php'));
        self::assertSame(HostRouting::PAGES, HostRouting::area('/generate/get-signature.php'));
        self::assertSame(HostRouting::LEGACY_ADMIN, HostRouting::area('/generate/stats.php'));
    }

    public function testQrHostWithoutTheTransitionFlag(): void
    {
        $settings = \Scutoons\Infrastructure\Config\Settings::fixed(self::root(), ['QR_HOST_SERVES_API' => '0'] + $this->settings());
        $routing = new HostRouting($settings, new \Slim\Psr7\Factory\ResponseFactory());
        self::assertNotContains(HostRouting::API, $routing->areasFor('qr.test'));
        self::assertContains(HostRouting::PUBLIC_API, $routing->areasFor('QR.TEST'));
        self::assertSame([HostRouting::API, HostRouting::PUBLIC_API], $routing->areasFor('api.test'));
        self::assertCount(4, $routing->areasFor('localhost'));
    }

    public function testLandingPageAndRedirects(): void
    {
        $promo = $this->code('promo');
        $r = $this->request('GET', "http://localhost/?c=$promo&lang=de");
        self::assertSame(200, $r->getStatusCode());
        self::assertStringContainsString('Promo-Code – kein Gegenstand zum Sammeln', (string) $r->getBody());
        self::assertStringContainsString("https://scutoons.com/account?code=$promo", (string) $r->getBody());
        $page = $this->code('page', ['redirect_to' => 'https://scutoons.com/catpope?x=1']);
        $r = $this->request('GET', "http://localhost/?c=$page");
        self::assertSame([302, "https://scutoons.com/catpope?x=1&code=$page"], [$r->getStatusCode(), $r->getHeaderLine('Location')]);
        self::assertSame(404, $this->request('GET', 'http://localhost/?c=zzzzzzzz')->getStatusCode());
        $r = $this->request('GET', 'http://localhost/?login=tok_en');
        self::assertSame('http://viewer.test/?login=tok_en', $r->getHeaderLine('Location'));
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM code_events WHERE code = ? AND event = 'scan'", [$promo]));
    }
}
