<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Cyberdecks (#180, agents/research/CYBERDECKS.md §9): handshake → approval by a member → short tokens from a signed
 * challenge; revoke / expiry → 401 and a new request; every member (readers too) pairs, a deck only reads with its
 * member's rights.
 */
final class DecksApiTest extends TestCase
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
     * @return array<string, string> the session header
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return ['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])];
    }

    /** @return array{0: string, 1: string} the public key (hex) and the secret key */
    private static function keys(): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        return [bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair)), ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)];
    }

    /** @return array<string, mixed> the deck's request answer */
    private function ask(string $public): array
    {
        $r = $this->request('POST', '/api/decks/request', [], ['public_key' => $public, 'build' => '2026.10.29-a1', 'class' => 'A', 'hostname' => 'x1-stage']);
        self::assertContains($r->getStatusCode(), [200, 201], (string) $r->getBody());
        return self::json($r)['deck'];
    }

    /** A challenge signed with $secret → the token response */
    private function renew(string $public, string $secret): ResponseInterface
    {
        $r = $this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]);
        if ($r->getStatusCode() !== 200) return $r;
        $challenge = (string) self::json($r)['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, $secret));
        return $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
    }

    /**
     * A paired deck: [public key, secret key, the deck's bearer header, the approved item]
     * @param array<string, string> $member
     * @param array<string, mixed> $approval
     * @return array{0: string, 1: string, 2: array<string, string>, 3: array<string, mixed>}
     */
    private function pair(array $member, array $approval = []): array
    {
        [$public, $secret] = self::keys();
        $code = (string) $this->ask($public)['code'];
        $r = $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code] + $approval);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $item = self::json($r)['item'];
        $t = $this->renew($public, $secret);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return [$public, $secret, ['Authorization' => 'Bearer ' . self::json($t)['token']], $item];
    }

    public function testHandshakeApprovalAndTokens(): void
    {
        $member = $this->session([], 'reader@scutoons.test');
        [$public, $secret] = self::keys();

        // no data before approval: pending, no challenge, no token
        $asked = $this->ask($public);
        self::assertSame('pending', $asked['status']);
        self::assertMatchesRegularExpression('/^[A-HJKMNP-Z2-9]{6}$/', $asked['code']);
        self::assertSame($asked['code'], $this->ask($public)['code'], 'asking again while it waits: the same code');
        $poll = $this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]);
        self::assertSame([202, 'pending'], [$poll->getStatusCode(), self::json($poll)['deck']['status']]);
        self::assertArrayNotHasKey('challenge', self::json($poll)['deck']);
        self::assertSame(401, $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => str_repeat('0', 64), 'signature' => str_repeat('0', 128)])->getStatusCode());

        // the pairing page shows the deck behind the code (typed with a dash, lower case), then the member approves
        $code = substr($asked['code'], 0, 3) . '-' . strtolower(substr($asked['code'], 3));
        $pending = self::json($this->request('GET', '/api/me/decks/pending?code=' . urlencode($code), $member));
        self::assertSame(['x1-stage', '2026.10.29-a1', 'A'], [$pending['deck']['hostname'], $pending['deck']['build'], $pending['deck']['class']]);
        $item = self::json($this->request('POST', '/api/me/decks/approve', $member, ['code' => $code, 'name' => 'Stage left']))['item'];
        self::assertSame(['Stage left', 'member', 182, null, $this->clock->utc(182 * 86400), false],
            [$item['name'], $item['mapping'], $item['lifetime_days'], $item['idle_days'], $item['expires_at'], $item['expired']]);
        self::assertArrayNotHasKey('scopes', $item);
        self::assertSame(404, $this->request('GET', '/api/me/decks/pending?code=' . $asked['code'], $member)->getStatusCode(), 'a code is used once');

        // the signed challenge → a short token, only its hash stored; the token is the member
        $t = $this->renew($public, $secret);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        $token = self::json($t);
        self::assertStringStartsWith('scd_', $token['token']);
        self::assertSame([$this->clock->utc(3600), 'Stage left'], [$token['expires_at'], $token['deck']['name']]);
        self::assertNull($this->db->value('SELECT id FROM decks WHERE token_hash = ?', [$token['token']]));
        $deck = ['Authorization' => 'Bearer ' . $token['token']];
        self::assertSame('reader@scutoons.test', self::json($this->request('GET', '/api/me', $deck))['user']['email']);
        self::assertSame(['approved'], array_column([$this->ask($public)], 'status'), 'an approved deck asking again keeps its approval');
        $list = self::json($this->request('GET', '/api/me/decks', $member))['items'];
        self::assertSame(['Stage left'], array_column($list, 'name'));

        // the token runs out after an hour; the deck signs a new challenge
        $this->clock->advance(3601);
        self::assertSame(401, $this->request('GET', '/api/me', $deck)->getStatusCode());
        self::assertSame(200, $this->renew($public, $secret)->getStatusCode());
    }

    public function testRevokeEndsTheNextRenewalAndTheDeckAsksAgain(): void
    {
        $member = $this->session([], 'reader@scutoons.test');
        [$public, $secret, $deck, $item] = $this->pair($member);
        self::assertSame(200, $this->request('GET', '/api/me', $deck)->getStatusCode());

        self::assertSame(200, $this->request('DELETE', "/api/me/decks/{$item['id']}", $member)->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/me', $deck)->getStatusCode(), 'its token stops at once');
        $r = $this->renew($public, $secret);
        self::assertSame([401, 'revoked'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        self::assertSame([], self::json($this->request('GET', '/api/me/decks', $member))['items']);
        self::assertSame(404, $this->request('DELETE', "/api/me/decks/{$item['id']}", $member)->getStatusCode());

        // a new request from the splash: the same row, a new code, nothing of the old approval
        $again = $this->ask($public);
        self::assertSame(['pending', $item['id']], [$again['status'], $again['id']]);
        self::assertNull($this->db->value('SELECT user_id FROM decks WHERE id = ?', [$item['id']]));
    }

    public function testExpiredApprovalIdleWindowAndRenew(): void
    {
        $member = $this->session([], 'reader@scutoons.test');
        [$public, $secret, , $item] = $this->pair($member, ['days' => 30]);
        self::assertSame($this->clock->utc(30 * 86400), $item['expires_at']);

        // its last two weeks: ONE reminder mail when the deck renews
        $this->clock->advance(20 * 86400);
        self::assertSame([200, 200], [$this->renew($public, $secret)->getStatusCode(), $this->renew($public, $secret)->getStatusCode()]);
        $log = is_file($this->dir . '/mail.log') ? (string) file_get_contents($this->dir . '/mail.log') : '';
        self::assertSame(1, substr_count($log, 'Subject: Your deck'), $log);

        // past its end → 401 expired; the member renews (a real sign-in) and the same deck goes on
        $this->clock->advance(11 * 86400);
        $r = $this->renew($public, $secret);
        self::assertSame([401, 'expired'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        self::assertTrue(self::json($this->request('GET', '/api/me/decks', $member))['items'][0]['expired']);
        $renewed = self::json($this->request('POST', "/api/me/decks/{$item['id']}/renew", $member))['item'];
        self::assertSame([$this->clock->utc(30 * 86400), false], [$renewed['expires_at'], $renewed['expired']]);
        self::assertSame(200, $this->renew($public, $secret)->getStatusCode());

        // idle: not seen for idle_days → ended
        [$idlePublic, $idleSecret] = $this->pair($member, ['idle_days' => 3]);
        $this->clock->advance(2 * 86400);
        self::assertSame(200, $this->renew($idlePublic, $idleSecret)->getStatusCode(), 'seen within the window: it moves on');
        $this->clock->advance(3 * 86400 + 60);
        self::assertSame(401, $this->renew($idlePublic, $idleSecret)->getStatusCode());
    }

    public function testExpiredPendingRequestWrongSignatureAndWrongCode(): void
    {
        $member = $this->session([], 'reader@scutoons.test');
        [$public, $secret] = self::keys();
        $code = (string) $this->ask($public)['code'];

        // a pending request ends after minutes
        $this->clock->advance(Decks::PENDING_MINUTES * 60 + 1);
        $r = $this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]);
        self::assertSame([401, 'request_expired'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        self::assertSame(404, $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());

        // a new request; approved – but signed with another key: 401, and the challenge is used up
        $code = (string) $this->ask($public)['code'];
        self::assertSame(200, $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());
        [, $otherSecret] = self::keys();
        $challenge = (string) self::json($this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $forged = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, $otherSecret));
        $r = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $forged]);
        self::assertSame([401, 'bad_signature'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        $good = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, $secret));
        $r = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $good]);
        self::assertSame(401, $r->getStatusCode(), 'a challenge is single use');
        self::assertSame(200, $this->renew($public, $secret)->getStatusCode());

        // bad input
        self::assertSame(400, $this->request('POST', '/api/decks/request', [], ['public_key' => 'nope'])->getStatusCode());
        self::assertSame(400, $this->request('POST', '/api/me/decks/approve', $member, ['code' => 'ABCDEF', 'days' => 400])->getStatusCode());
        // wrong codes are limited per member (the expired one above counted too)
        for ($i = 1; $i < Decks::CODE_TRIES; $i++) {
            self::assertSame(404, $this->request('GET', '/api/me/decks/pending?code=ZZZZZZ', $member)->getStatusCode());
        }
        self::assertSame(429, $this->request('GET', '/api/me/decks/pending?code=ZZZZZZ', $member)->getStatusCode());
    }

    public function testRequestsAreRateLimitedPerIp(): void
    {
        for ($i = 0; $i < 30; $i++) {
            self::assertSame(201, $this->request('POST', '/api/decks/request', [], ['public_key' => bin2hex(random_bytes(32))])->getStatusCode());
        }
        $r = $this->request('POST', '/api/decks/request', [], ['public_key' => bin2hex(random_bytes(32))]);
        self::assertSame(429, $r->getStatusCode(), (string) $r->getBody());
    }

    public function testADeckReadsWithItsMembersRightsOnly(): void
    {
        // a reader's deck: the reader's reads, nothing more; no writes at all
        $reader = $this->session([], 'reader@scutoons.test');
        [, , $deck] = $this->pair($reader);
        self::assertSame('reader@scutoons.test', self::json($this->request('GET', '/api/me', $deck))['user']['email']);
        self::assertSame(403, $this->request('GET', '/api/admin/users', $deck)->getStatusCode());
        foreach ([['PUT', '/api/me'], ['POST', '/api/inventory/claim'], ['GET', '/api/me/decks'], ['GET', '/api/me/tokens'], ['POST', '/api/auth/logout']] as [$method, $path]) {
            $r = $this->request($method, $path, $deck, []);
            self::assertSame([403, 'not_for_deck'], [$r->getStatusCode(), self::json($r)['error'] ?? null], "$method $path");
        }

        // an admin's deck reads what the admin reads – still no writes
        $admin = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        [, , $adminDeck] = $this->pair($admin);
        self::assertSame(200, $this->request('GET', '/api/admin/users', $adminDeck)->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/admin/proposals', $adminDeck, [])->getStatusCode());

        // managing decks is a real sign-in: an AI connection can't
        $creator = $this->session(['creator'], 'creator@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . self::json($this->request('POST', '/api/me/tokens', $creator, ['name' => 'Claude']))['token']];
        self::assertSame(403, $this->request('POST', '/api/me/decks/approve', $ai, ['code' => 'ABCDEF'])->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/me/decks', $ai)->getStatusCode());
    }
}
