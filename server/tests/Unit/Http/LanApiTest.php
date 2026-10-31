<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Kas\KasApi;
use Scutoons\Kernel;
use Scutoons\Tests\Domain\LanCertTest;
use Scutoons\Tests\Infrastructure\FakeKas;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * The venue relay's names and certificate through the API (#201, Admin\LanController): admins only; without KAS
 * credentials 503; names and addresses checked before all-inkl is asked (FakeKas – never the real zone); the workflow
 * stores the certificate with the deploy key, an admin's AI connection (deck-tls) reads it, as JSON or PEM, but cannot
 * set names (admin writes stay the member's click).
 */
final class LanApiTest extends TestCase
{
    private const KEY = 'kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk';
    private FakeKas $kas;
    private bool $configured = true;

    protected function setUp(): void
    {
        parent::setUp();
        $this->kas = new FakeKas();
    }

    protected function settings(): array
    {
        $kas = $this->configured ? ['KAS_LOGIN' => 'w0123456', 'KAS_AUTH_DATA' => 'secret'] : [];
        return $kas + ['MIGRATE_KEY' => self::KEY, 'LAN_CERT_PATH' => $this->dir . '/lan-cert'] + parent::settings();
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
        $container->set(KasApi::class, $this->kas);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param list<string> $roles
     * @return array{array<string, string>, array<string, mixed>} the session header + the user
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    public function testAdminsSetAndListNames(): void
    {
        [$admin] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        $r = $this->request('PUT', '/api/admin/lan-names/x1', $admin, ['ip' => '192.168.1.116']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['name' => 'x1', 'host' => 'x1.lan.scutoons.com', 'ip' => '192.168.1.116', 'change' => 'added'], self::json($r)['item']);

        $r = $this->request('GET', '/api/admin/lan-names', $admin);
        self::assertSame('lan.scutoons.com', self::json($r)['domain']);
        self::assertSame(['old', 'x1'], array_column(self::json($r)['items'], 'name'));

        foreach ([['x1', ['ip' => '8.8.8.8'], 'invalid_ip'], ['x1', ['ip' => 7], 'invalid_ip'], ['bad_name', ['ip' => '10.0.0.2'], 'invalid_name']] as [$name, $body, $error]) {
            $r = $this->request('PUT', "/api/admin/lan-names/$name", $admin, $body);
            self::assertSame([422, $error], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        }

        $this->kas->fail = 'flood_protection';
        $r = $this->request('GET', '/api/admin/lan-names', $admin);
        self::assertSame([502, 'kas_error'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testOnlyAdminsAndNotConfigured503(): void
    {
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        self::assertSame(403, $this->request('GET', '/api/admin/lan-names', $creator)->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/admin/lan-names/x1', $creator, ['ip' => '192.168.1.2'])->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/admin/lan-cert', $creator)->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/admin/lan-cert')->getStatusCode());

        $this->configured = false;
        [$admin] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        $r = $this->request('GET', '/api/admin/lan-names', $admin);
        self::assertSame([503, 'not_configured'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame([], $this->kas->calls);
    }

    public function testTheWorkflowStoresTheCertificateAndDecksFetchIt(): void
    {
        [$pem, $key] = LanCertTest::pair('*.lan.scutoons.com');
        [$admin, $user] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        $r = $this->request('GET', '/api/admin/lan-cert', $admin);
        self::assertSame([404, 'no_certificate'], [$r->getStatusCode(), self::json($r)['error']]);

        $body = ['fullchain' => $pem, 'privkey' => $key];
        self::assertSame(403, $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => 'wrong'], $body)->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/deploy/lan-cert', [], $body)->getStatusCode());
        $r = $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], $body);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['*.lan.scutoons.com'], self::json($r)['names']);
        $r = $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], ['fullchain' => $pem]);
        self::assertSame([422, 'invalid_body'], [$r->getStatusCode(), self::json($r)['error']]);

        // deck-tls: an admin's AI connection reads it – as JSON, and as one PEM text (chain, then key)
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'deck-tls on the X1')['token']];
        $r = $this->request('GET', '/api/admin/lan-cert', $ai);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(trim($key), trim((string) self::json($r)['privkey']));
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        $r = $this->request('GET', '/api/admin/lan-cert?format=pem', $ai);
        self::assertSame('application/x-pem-file', $r->getHeaderLine('Content-Type'));
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        self::assertSame(trim($pem) . "\n" . trim($key) . "\n", (string) $r->getBody());
        // … but an AI connection never sets a name (admin writes are the member's click – ApiTokenScope)
        $r = $this->request('PUT', '/api/admin/lan-names/x1', $ai, ['ip' => '192.168.1.2']);
        self::assertSame([403, 'not_for_ai'], [$r->getStatusCode(), self::json($r)['error']]);
    }

    public function testWithoutAMigrateKeyTheUploadIsUnknown(): void
    {
        [$pem, $key] = LanCertTest::pair('*.lan.scutoons.com');
        $request = (new ServerRequestFactory())->createServerRequest('PUT', 'http://localhost/api/deploy/lan-cert', ['REMOTE_ADDR' => '127.0.0.1'])
            ->withHeader('X-Migrate-Key', self::KEY)->withHeader('Content-Type', 'application/json');
        $request->getBody()->write((string) json_encode(['fullchain' => $pem, 'privkey' => $key]));
        $request->getBody()->rewind();
        $container = Kernel::container(Settings::fixed(self::root(), ['MIGRATE_KEY' => ''] + $this->settings()));
        self::assertSame(404, Kernel::app($container)->handle($request)->getStatusCode());
    }
}
