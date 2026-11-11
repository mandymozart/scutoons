<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Live\LiveSessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * A client's proof for a live session (#478, LiveProofController): a member's account roles among member, creator, admin,
 * live; an approved deck's own token `approved` and its member's roles (#481); nothing about who it is. The shared vector with the relay:
 * deck/protocol/fixtures/live-proof.json.
 */
final class LiveProofApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['LIVE_TOKEN_SECRET' => 'proof-test-secret'] + parent::settings();
    }

    public function testAMemberGetsItsAccountFactsAndNothingElse(): void
    {
        [$member] = $this->session([Roles::CREATOR, Roles::LIVE, Roles::DEVELOPER], 'anna@scutoons.test');
        $r = $this->request('GET', '/api/live/proof?session=A7K2', $member);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        $body = self::json($r);
        self::assertSame(LiveSessions::PROOF_SECONDS, $body['expires_in']);
        self::assertFalse($body['dev']);
        $claims = self::claims($body['proof'], 'proof-test-secret');
        // the facts only: no account id, no name, no mail; developer is no fact
        self::assertSame(['session' => 'A7K2', 'facts' => ['account' => ['member', 'creator', 'live'], 'approved' => false], 'exp' => $this->clock->time() + 3600], $claims);
        self::assertStringNotContainsString('anna', (string) $r->getBody());
    }

    public function testAnApprovedDeckIsApprovedAndCarriesItsMembersAccountFacts(): void
    {
        // #481: a paired deck is tied to an account – its proof says that account's roles (whether its person may reject a
        // flash is one), and still nothing about who it is
        [$member] = $this->session([Roles::LIVE, Roles::DEVELOPER], 'owner@scutoons.test');
        $r = $this->request('GET', '/api/live/proof?session=CATH', $this->deck($member));
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['account' => ['member', 'live'], 'approved' => true], self::claims(self::json($r)['proof'], 'proof-test-secret')['facts']);
        self::assertStringNotContainsString('owner', (string) $r->getBody());
        // a deck of a member without `live`: approved, and a member
        [$plain] = $this->session([], 'guest@scutoons.test');
        $r = $this->request('GET', '/api/live/proof?session=CATH', $this->deck($plain));
        self::assertSame(['account' => ['member'], 'approved' => true], self::claims(self::json($r)['proof'], 'proof-test-secret')['facts']);
    }

    public function testRefusals(): void
    {
        self::assertSame(401, $this->request('GET', '/api/live/proof?session=A7K2')->getStatusCode(), 'nobody: a guest needs no proof');
        [$member, $user] = $this->session([], 'reader@scutoons.test');
        self::assertSame(400, $this->request('GET', '/api/live/proof', $member)->getStatusCode());
        self::assertSame(400, $this->request('GET', '/api/live/proof?session=' . rawurlencode('A7/../x'), $member)->getStatusCode());
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $ai = $this->get(ApiTokens::class)->create($user, 'claude')['token'];
        $r = $this->request('GET', '/api/live/proof?session=A7K2', ['Authorization' => "Bearer $ai"]);
        self::assertSame(403, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('not_for_ai', self::json($r)['error']);
    }

    public function testWithoutASecretTheFactsAreSignedWithTheEmptyKey(): void
    {
        [$member] = $this->session([], 'dev@scutoons.test');
        $body = self::json($this->request('GET', '/api/live/proof?session=A7K2', $member, ['LIVE_TOKEN_SECRET' => '']));
        self::assertTrue($body['dev']);
        self::assertSame(['member'], self::claims($body['proof'], '')['facts']['account']);
    }

    public function testTheProofIsWhatTheRelayExpects(): void
    {
        // the same file as deck/relay/src/token.rs `the_shared_vector`
        $v = json_decode((string) file_get_contents(self::root() . '/../deck/protocol/fixtures/live-proof.json'), true);
        self::assertIsArray($v);
        self::assertSame($v['proof'], LiveSessions::sign($v['secret'], $v['claims']));
        self::assertSame($v['facts'], self::claims($v['proof'], $v['secret'])['facts']);
    }

    /**
     * The claims of a signed proof, its signature checked
     * @return array<string, mixed>
     */
    private static function claims(string $proof, string $secret): array
    {
        [$payload, $sig] = explode('.', $proof);
        self::assertSame(rtrim(strtr(base64_encode(hash_hmac('sha256', $payload, $secret, true)), '+/', '-_'), '='), $sig);
        return (array) json_decode((string) base64_decode(strtr($payload, '-_', '+/')), true);
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $settings
     */
    private function request(string $method, string $uri, array $headers = [], array $settings = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        $container = Kernel::container(Settings::fixed(self::root(), $settings + $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /**
     * POST with a JSON body (the deck's handshake)
     * @param array<string, string> $headers
     * @param array<string, mixed> $body
     */
    private function post(string $uri, array $headers, array $body): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('POST', "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        $request->getBody()->write((string) json_encode($body));
        $request->getBody()->rewind();
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request->withHeader('Content-Type', 'application/json'));
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /**
     * A signed-in account with `member` and these roles → its bearer header and its row
     * @param list<string> $roles
     * @return array{0: array<string, string>, 1: array<string, mixed>}
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ([Roles::MEMBER, ...$roles] as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    /**
     * A deck paired by $member (deck-link's handshake, as DeckMediaApiTest) → its bearer header
     * @param array<string, string> $member
     * @return array<string, string>
     */
    private function deck(array $member): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $code = (string) self::json($this->post('/api/decks/request', [], ['public_key' => $public, 'hostname' => 'deck']))['deck']['code'];
        self::assertSame(200, $this->post('/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->post('/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->post('/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }
}
