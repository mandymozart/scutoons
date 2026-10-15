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

/** The HTTP side of the admin code page: change or remove a code's expiry (PUT /api/admin/codes/{code}/expiry) */
final class CodesAdminApiTest extends TestCase
{
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
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * @param array<string, mixed> $user
     * @return array<string, string>
     */
    private function auth(array $user): array
    {
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id']), 'Content-Type' => 'application/json'];
    }

    public function testAnAdminChangesAndRemovesTheExpiry(): void
    {
        $code = $this->code('promo', ['code' => '1lA_xqs7', 'expires_at' => '2026-12-31 23:59:59']);
        $uri = "/api/admin/codes/$code/expiry";
        $body = (string) json_encode(['expires_at' => '2027-06-30T23:59']);

        self::assertSame(401, $this->request('PUT', $uri, ['Content-Type' => 'application/json'], $body)->getStatusCode());
        $member = $this->auth($this->user('member@example.com'));
        self::assertSame(403, $this->request('PUT', $uri, $member, $body)->getStatusCode());

        $boss = $this->user('boss@example.com');
        $this->get(Roles::class)->grant((string) $boss['id'], Roles::ADMIN);
        $admin = $this->auth($boss);
        $r = $this->request('PUT', $uri, $admin, $body);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['success', $code, '2027-06-30 23:59:00', false], [self::json($r)['status'], self::json($r)['code'], self::json($r)['expires_at'], self::json($r)['expired']]);
        $detail = self::json($this->request('GET', "/api/admin/codes/$code", $admin));
        self::assertSame('2027-06-30 23:59:00', $detail['code']['expires_at']);
        $events = self::json($this->request('GET', "/api/admin/codes/$code/events", $admin));
        self::assertSame(['expiry'], array_column($events['items'], 'event'));

        // null = no expiry; a missing field or a non-date is a 400; an unknown code a 404
        $r = $this->request('PUT', $uri, $admin, '{"expires_at":null}');
        self::assertSame([200, null], [$r->getStatusCode(), self::json($r)['expires_at']]);
        self::assertSame('invalid_expires_at', self::json($this->request('PUT', $uri, $admin, '{}'))['error']);
        self::assertSame(400, $this->request('PUT', $uri, $admin, '{"expires_at":"soon"}')->getStatusCode());
        self::assertSame(404, $this->request('PUT', '/api/admin/codes/nope0000/expiry', $admin, $body)->getStatusCode());
    }
}
