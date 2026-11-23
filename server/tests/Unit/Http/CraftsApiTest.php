<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\LoginService;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Crafts (Domain\Crafts, #826): the seeded list, a member's own pick only at the first login, admins any time, an admin's
 * invite that carries crafts and applies them when the address signs up the normal way – it signs no one up.
 */
final class CraftsApiTest extends TestCase
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
     * @return array{0: array<string, string>, 1: string}
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    public function testTheListIsSeededWithWorkspaces(): void
    {
        [$member] = $this->session([], 'list@scutoons.test');
        self::assertSame(401, $this->request('GET', '/api/crafts')->getStatusCode());
        $crafts = self::json($this->request('GET', '/api/crafts', $member))['crafts'];
        self::assertSame(['show_runner', 'illustrator', 'video_editor', 'writer', 'translator', 'animator', 'sound_designer', 'developer'], array_column($crafts, 'id'));
        $illustrator = $crafts[1];
        self::assertSame(['Illustrator', 'pen-tool', ['character', 'storyboard', 'chapter']], [$illustrator['name']['en'], $illustrator['icon'], $illustrator['workspaces']]);
    }

    public function testAMemberPicksOnceAtTheFirstLogin(): void
    {
        [$member] = $this->session(['creator'], 'mira@scutoons.test');
        $state = self::json($this->request('GET', '/api/me/crafts', $member));
        self::assertSame([[], [], true, true], [$state['crafts'], $state['workspaces'], $state['ask'], $state['can_edit']]);

        $r = $this->request('PUT', '/api/me/crafts', $member, ['crafts' => ['translator', 'illustrator']]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $state = self::json($r);
        // the crafts' order, the workspaces resolved by the server, each once
        self::assertSame(['illustrator', 'translator'], $state['crafts']);
        self::assertSame(['character', 'storyboard', 'chapter', 'text'], $state['workspaces']);
        self::assertSame([false, false], [$state['ask'], $state['can_edit']]);

        // answered: the member can't change them any more
        $r = $this->request('PUT', '/api/me/crafts', $member, ['crafts' => ['writer']]);
        self::assertSame([403, 'crafts_locked'], [$r->getStatusCode(), self::json($r)['code'] ?? self::json($r)['error'] ?? null]);
        self::assertSame(['illustrator', 'translator'], self::json($this->request('GET', '/api/me/crafts', $member))['crafts']);
    }

    public function testNothingPickedStillAnswersTheQuestion(): void
    {
        [$member] = $this->session([], 'none@scutoons.test');
        self::assertSame(200, $this->request('PUT', '/api/me/crafts', $member, ['crafts' => []])->getStatusCode());
        self::assertFalse(self::json($this->request('GET', '/api/me/crafts', $member))['ask']);
    }

    public function testBadCraftsAreRefused(): void
    {
        [$member] = $this->session([], 'bad@scutoons.test');
        self::assertSame(400, $this->request('PUT', '/api/me/crafts', $member, ['crafts' => ['crafting']])->getStatusCode());
        self::assertSame(400, $this->request('PUT', '/api/me/crafts', $member, ['crafts' => 'writer'])->getStatusCode());
        self::assertTrue(self::json($this->request('GET', '/api/me/crafts', $member))['ask']);
    }

    public function testAnAdminChangesOwnCraftsAndAnyAccounts(): void
    {
        [$admin] = $this->session(['admin'], 'admin@scutoons.test');
        [$member, $memberId] = $this->session(['creator'], 'cleo@scutoons.test');
        self::assertSame(200, $this->request('PUT', '/api/me/crafts', $admin, ['crafts' => ['developer']])->getStatusCode());
        self::assertTrue(self::json($this->request('GET', '/api/me/crafts', $admin))['can_edit']);
        self::assertSame(200, $this->request('PUT', '/api/me/crafts', $admin, ['crafts' => ['developer', 'writer']])->getStatusCode());

        $r = $this->request('PUT', "/api/admin/users/$memberId/crafts", $admin, ['crafts' => ['sound_designer']]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame([['id' => 'sound_designer', 'source' => 'admin']], self::json($r)['crafts']);
        self::assertSame([['id' => 'sound_designer', 'source' => 'admin']], self::json($this->request('GET', "/api/admin/users/$memberId", $admin))['user']['crafts']);
        // the member still gets the question, with the admin's craft ticked
        $state = self::json($this->request('GET', '/api/me/crafts', $member));
        self::assertSame([['sound_designer'], true], [$state['crafts'], $state['ask']]);

        // not admins: no admin route
        self::assertSame(403, $this->request('PUT', "/api/admin/users/$memberId/crafts", $member, ['crafts' => []])->getStatusCode());
        self::assertSame(404, $this->request('PUT', '/api/admin/users/00000000-0000-0000-0000-000000000000/crafts', $admin, ['crafts' => []])->getStatusCode());
    }

    public function testAnInviteCarriesCraftsAndAppliesThemAtTheNormalSignup(): void
    {
        [$admin] = $this->session(['admin'], 'admin@scutoons.test');
        $r = $this->request('POST', '/api/admin/member-invites', $admin, ['email' => 'New@Scutoons.test', 'crafts' => ['translator', 'illustrator']]);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $invite = self::json($r)['invite'];
        self::assertSame(['new@scutoons.test', ['illustrator', 'translator'], null], [$invite['email'], $invite['crafts'], $invite['accepted_at']]);
        // a second invite to the address replaces the crafts
        self::assertSame(201, $this->request('POST', '/api/admin/member-invites', $admin, ['email' => 'new@scutoons.test', 'crafts' => ['translator', 'animator']])->getStatusCode());
        self::assertCount(1, self::json($this->request('GET', '/api/admin/member-invites', $admin))['items']);
        // an address with an account: the user page
        $this->user('known@scutoons.test');
        self::assertSame(409, $this->request('POST', '/api/admin/member-invites', $admin, ['email' => 'known@scutoons.test', 'crafts' => []])->getStatusCode());

        // the invite is no signup code: without one the invited address is refused as anyone
        $login = $this->get(LoginService::class);
        try {
            $login->confirm($this->loginRequest('new@scutoons.test'), []);
            self::fail('an invited address signed up without a code');
        } catch (\Scutoons\Domain\ApiError $e) {
            self::assertSame('signup_code_required', $e->errorCode);
        }
        self::assertNull($this->db->one('SELECT id FROM users WHERE email = ?', ['new@scutoons.test']));
        self::assertNull(self::json($this->request('GET', '/api/admin/member-invites', $admin))['items'][0]['accepted_at']);

        // with a signup code the account comes, and the invite's crafts with it
        $result = $login->confirm($this->loginRequest('new@scutoons.test', $this->code('promo')), []);
        self::assertTrue($result['created']);
        $user = $this->db->one('SELECT * FROM users WHERE email = ?', ['new@scutoons.test']);
        self::assertNotNull($user);
        $session = ['Authorization' => 'Bearer ' . $result['session_token']];
        $state = self::json($this->request('GET', '/api/me/crafts', $session));
        self::assertSame([['translator', 'animator'], ['translator', 'animator'], true], [$state['crafts'], $state['invited'], $state['ask']]);
        $items = self::json($this->request('GET', '/api/admin/member-invites', $admin))['items'];
        self::assertSame($user['id'], $items[0]['user_id']);
        self::assertNotNull($items[0]['accepted_at']);
        // accepted: no withdrawing
        self::assertSame(409, $this->request('DELETE', "/api/admin/member-invites/{$items[0]['id']}", $admin)->getStatusCode());
    }

    public function testAnInviteForAnAccountMadeMeanwhileArrivesAtItsSignIn(): void
    {
        [$admin] = $this->session(['admin'], 'admin@scutoons.test');
        self::assertSame(201, $this->request('POST', '/api/admin/member-invites', $admin, ['email' => 'late@scutoons.test', 'crafts' => ['writer']])->getStatusCode());
        $user = $this->user('late@scutoons.test');
        $result = $this->get(LoginService::class)->confirm($this->loginRequest('late@scutoons.test'), []);
        self::assertFalse($result['created']);
        self::assertSame([['id' => 'writer', 'source' => 'invite']], self::json($this->request('GET', "/api/admin/users/{$user['id']}", $admin))['user']['crafts']);
    }

    public function testAnOpenInviteCanBeWithdrawn(): void
    {
        [$admin] = $this->session(['admin'], 'admin@scutoons.test');
        $id = self::json($this->request('POST', '/api/admin/member-invites', $admin, ['email' => 'gone@scutoons.test', 'crafts' => ['writer']]))['invite']['id'];
        self::assertSame(200, $this->request('DELETE', "/api/admin/member-invites/$id", $admin)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', '/api/admin/member-invites', $admin))['items']);
        self::assertSame(404, $this->request('DELETE', "/api/admin/member-invites/$id", $admin)->getStatusCode());
    }

    public function testAnAiConnectionCannotAnswerForTheMember(): void
    {
        [$member] = $this->session(['creator'], 'ai@scutoons.test');
        $token = (string) self::json($this->request('POST', '/api/me/tokens', $member, ['name' => 'Claude']))['token'];
        $ai = ['Authorization' => 'Bearer ' . $token];
        self::assertSame(200, $this->request('GET', '/api/me/crafts', $ai)->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/me/crafts', $ai, ['crafts' => ['writer']])->getStatusCode());
    }

    /** @return array<string, mixed> a fresh login_requests row for the address (with a signup code or none) */
    private function loginRequest(string $email, ?string $code = null): array
    {
        $id = \Scutoons\Support\Ids::uuid();
        $this->db->run('INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, ip_hash, attempts, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)',
            [$id, $email, hash('sha256', $id), hash('sha256', 'otp' . $id), 'en', $code, 'test', $this->clock->utc(), $this->clock->utc(900)]);
        return $this->db->one('SELECT * FROM login_requests WHERE id = ?', [$id]) ?? [];
    }
}
