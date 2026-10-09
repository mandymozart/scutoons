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

/**
 * AI connections (personal access tokens, agents/MCP.md §3): members only, shown once, the member's current rights,
 * revocable, signed saves – and refused on what only the member does in the app (ApiTokenScope).
 */
final class ApiTokensApiTest extends TestCase
{
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

    /**
     * @param list<string> $roles
     * @return array{0: array<string, string>, 1: string} the session header and the user id
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    /** @param array<string, string> $session */
    private function token(array $session, string $name = 'Claude on my laptop'): string
    {
        $r = $this->request('POST', '/api/me/tokens', $session, ['name' => $name]);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        return (string) self::json($r)['token'];
    }

    public function testMembersCreateListUseAndRevoke(): void
    {
        [$creator] = $this->session(['creator'], 'creator@scutoons.test');
        $r = $this->request('POST', '/api/me/tokens', $creator, ['name' => 'Claude on my laptop']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $created = self::json($r);
        self::assertStringStartsWith('sct_', $created['token']);
        // only the hash is stored
        self::assertNull($this->db->one('SELECT id FROM api_tokens WHERE token_hash = ?', [$created['token']]));
        $list = self::json($this->request('GET', '/api/me/tokens', $creator));
        self::assertSame([true, ['Claude on my laptop'], null], [$list['member'], array_column($list['items'], 'name'), $list['items'][0]['last_used_at']]);
        self::assertArrayNotHasKey('token', $list['items'][0]);

        // the token is the member: /me, the content API
        $ai = ['Authorization' => 'Bearer ' . $created['token']];
        self::assertSame('creator@scutoons.test', self::json($this->request('GET', '/api/me', $ai))['user']['email']);
        self::assertNotNull(self::json($this->request('GET', '/api/me/tokens', $creator))['items'][0]['last_used_at']);

        // a save is signed with the connection's name
        $r = $this->request('PUT', '/api/entities/moods', $ai, ['content' => ['moods' => [['id' => 'happy', 'label' => ['en' => 'Happy']]]], 'base' => null, 'message' => 'fix']);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('fix (AI: Claude on my laptop)', $this->db->value('SELECT message FROM entity_versions ORDER BY id DESC LIMIT 1'));

        // revoked → 401 at once
        self::assertSame(200, $this->request('DELETE', "/api/me/tokens/{$created['item']['id']}", $creator)->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/me', $ai)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', '/api/me/tokens', $creator))['items']);
        self::assertSame(404, $this->request('DELETE', "/api/me/tokens/{$created['item']['id']}", $creator)->getStatusCode());
    }

    public function testReadersCannotConnectAndALostRoleEndsTheToken(): void
    {
        [$reader] = $this->session([], 'reader@scutoons.test');
        self::assertFalse(self::json($this->request('GET', '/api/me/tokens', $reader))['member']);
        self::assertSame(403, $this->request('POST', '/api/me/tokens', $reader, ['name' => 'x'])->getStatusCode());
        [$liaison, $id] = $this->session(['liaison'], 'liaison@scutoons.test');
        self::assertSame(400, $this->request('POST', '/api/me/tokens', $liaison, ['name' => ' '])->getStatusCode());
        $ai = ['Authorization' => 'Bearer ' . $this->token($liaison)];
        self::assertSame(200, $this->request('GET', '/api/embassy/me', $ai)->getStatusCode());
        $this->get(Roles::class)->revoke($id, 'liaison');
        self::assertSame(401, $this->request('GET', '/api/me', $ai)->getStatusCode());
    }

    public function testWhatOnlyTheMemberDoesIsRefused(): void
    {
        [$admin] = $this->session([Roles::ADMIN, 'creator'], 'admin@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . $this->token($admin)];
        $refused = [
            ['POST', '/api/auth/session'],
            ['POST', '/api/auth/logout'],
            ['GET', '/api/me/tokens'],
            ['POST', '/api/me/tokens'],
            ['POST', '/api/entities/moods/publish'],
            ['POST', '/api/reviews/publish'],
            ['POST', '/api/embassy/orgs/01a10b79-f3c4-7aaf-8d00-2402d01438a4/mail'],
            ['PUT', '/api/admin/catalog/items/camera'],
            ['PUT', '/api/admin/proposals/01a10b79-f3c4-7aaf-8d00-2402d01438a4'],
            ['POST', '/api/admin/codes'],
        ];
        foreach ($refused as [$method, $path]) {
            $r = $this->request($method, $path, $ai, []);
            self::assertSame([403, 'not_for_ai'], [$r->getStatusCode(), self::json($r)['error'] ?? null], "$method $path");
        }
        // reading the admin data and proposing stay possible
        self::assertSame(200, $this->request('GET', '/api/admin/dashboard', $ai)->getStatusCode());
        self::assertSame(201, $this->request('POST', '/api/admin/proposals', $ai, ['title' => 'x', 'changes' => [['type' => 'asset', 'path' => 'a.webp', 'source' => null, 'usages' => 0]]])->getStatusCode());
    }
}
