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

    public function testSixMonthsOneReminderRenew(): void
    {
        [$creator] = $this->session(['creator'], 'creator@scutoons.test');
        $r = self::json($this->request('POST', '/api/me/tokens', $creator, ['name' => 'Claude Code']));
        $ai = ['Authorization' => 'Bearer ' . $r['token']];
        $id = $r['item']['id'];
        self::assertSame([$this->clock->utc(182 * 86400), false], [$r['item']['expires_at'], $r['item']['expired']]);

        // used in its last two weeks: ONE reminder mail to the member
        $this->db->run('UPDATE api_tokens SET expires_at = ? WHERE id = ?', [$this->clock->utc(10 * 86400), $id]);
        // twice: the second call sends nothing
        self::assertSame([200, 200], [$this->request('GET', '/api/me', $ai)->getStatusCode(), $this->request('GET', '/api/me', $ai)->getStatusCode()]);
        $log = is_file($this->dir . '/mail.log') ? (string) file_get_contents($this->dir . '/mail.log') : '';
        self::assertSame(1, substr_count($log, 'Subject: Your AI connection'), $log);
        self::assertStringContainsString('creator@scutoons.test', $log);

        // expired → 401; renewing (a real sign-in) brings the SAME token back for six months, the reminder may come again
        $this->db->run('UPDATE api_tokens SET expires_at = ? WHERE id = ?', [$this->clock->utc(-60), $id]);
        self::assertSame(401, $this->request('GET', '/api/me', $ai)->getStatusCode());
        self::assertTrue(self::json($this->request('GET', '/api/me/tokens', $creator))['items'][0]['expired']);
        self::assertSame(403, $this->request('POST', "/api/me/tokens/$id/renew", $ai)->getStatusCode());
        $renewed = self::json($this->request('POST', "/api/me/tokens/$id/renew", $creator))['item'];
        self::assertSame([$this->clock->utc(182 * 86400), false], [$renewed['expires_at'], $renewed['expired']]);
        self::assertNull($this->db->value('SELECT reminded_at FROM api_tokens WHERE id = ?', [$id]));
        self::assertSame(200, $this->request('GET', '/api/me', $ai)->getStatusCode());
    }

    public function testWhatOnlyTheMemberDoesIsRefused(): void
    {
        [$admin] = $this->session([Roles::ADMIN, 'creator'], 'admin@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . $this->token($admin)];
        $id = '01a10b79-f3c4-7aaf-8d00-2402d01438a4';
        $refused = [
            ['POST', '/api/auth/session'],
            ['POST', '/api/auth/logout'],
            ['GET', '/api/me/tokens'],
            ['POST', '/api/me/tokens'],
            ['POST', "/api/me/tokens/$id/renew"],
            ['DELETE', "/api/me/tokens/$id"],
            ['POST', '/api/entities/moods/publish'],
            ['POST', '/api/entities/moods/unpublish'],
            // the batch publish (#100)
            ['POST', '/api/entities/publish'],
            ['POST', '/api/entities/publish/'],
            ['POST', '/api/reviews/publish'],
            ['POST', '/api/reviews/return'],
            // new bytes under a published file reach readers at once (agents/MCP.md §2: replace-in-place = a proposal)
            ['PUT', '/api/assets/replace?path=catpope/images/x.png'],
            ['POST', "/api/embassy/orgs/$id/mail"],
            ['DELETE', "/api/embassy/orgs/$id"],
            ['POST', '/api/embassy/import'],
            ['PUT', '/api/admin/catalog/items/camera'],
            ['PUT', "/api/admin/proposals/$id"],
            ['POST', '/api/admin/codes'],
            // routing decodes %xx (Slim's RouteResolver) – so does the guard
            ['POST', '/api/entities/%70ublish'],
            ['POST', '/api/entities/moods/%70ublish'],
            ['PUT', '/api/%61dmin/catalog/items/camera'],
            ['PUT', '/api%2Fadmin/catalog/items/camera'],
        ];
        $answers = [];
        foreach ($refused as [$method, $path]) {
            $r = $this->request($method, $path, $ai, []);
            $answers["$method $path"] = $r->getStatusCode() . ' ' . (self::json($r)['error'] ?? '');
        }
        self::assertSame(array_fill_keys(array_keys($answers), '403 not_for_ai'), $answers);
        // a draft and its restore stay possible (the creator publishes), so do reading the admin data and proposing
        $r = $this->request('PUT', '/api/entities/moods', $ai, ['content' => ['moods' => [['id' => 'happy', 'label' => ['en' => 'Happy']]]]]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $r = $this->request('POST', '/api/entities/moods/restore', $ai, ['id' => self::json($r)['version']['id']]);
        self::assertSame([200, 'draft'], [$r->getStatusCode(), self::json($r)['version']['status'] ?? null], (string) $r->getBody());
        self::assertSame(200, $this->request('GET', '/api/admin/dashboard', $ai)->getStatusCode());
        self::assertSame(201, $this->request('POST', '/api/admin/proposals', $ai, ['title' => 'x', 'changes' => [['type' => 'asset', 'path' => 'a.webp', 'source' => null, 'usages' => 0]]])->getStatusCode());
    }

    /** invites and gifts are the member's rights through their AI too – the admin's campaigns and the limits decide (owner 2026-10-21) */
    public function testInvitesAndGiftsAreTheMembersRights(): void
    {
        [$creator] = $this->session(['creator'], 'creator@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . $this->token($creator)];
        $answers = [];
        foreach (['/api/invites', '/api/inventory/gift'] as $path) {
            $r = $this->request('POST', $path, $ai, []);
            $answers[$path] = $r->getStatusCode() . ' ' . (self::json($r)['error'] ?? '');
        }
        // the endpoint answers (it wants a recipient), not the token rule
        self::assertSame(['/api/invites' => '400 invalid_email', '/api/inventory/gift' => '400 invalid_email'], $answers);
    }
}
