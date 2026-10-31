<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\Domain\LanCertTest;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * The venue certificate through the API (#201, LanController): the workflow stores it with the deploy key; only an
 * approved deck's own token reads it (#180, owner 2026-10-31: no admin tokens on decks) – as JSON or PEM; a session or
 * an AI connection (an admin's too) gets 403, no token / a pending, revoked or unknown deck 401.
 */
final class LanApiTest extends TestCase
{
    private const KEY = 'kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk';

    protected function settings(): array
    {
        return ['MIGRATE_KEY' => self::KEY, 'LAN_CERT_PATH' => $this->dir . '/lan-cert'] + parent::settings();
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $settings
     */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, array $settings = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $settings + $this->settings()));
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
     * @return array{array<string, string>, array<string, mixed>} the session header + the user
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    /**
     * A deck asks for access (deck-link's handshake, DecksApiTest); approved by $member when given → its bearer header
     * @param array<string, string>|null $member
     * @return array{0: ?array<string, string>, 1: string} the deck's header (null while pending) + its code
     */
    private function deck(?array $member): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $r = $this->request('POST', '/api/decks/request', [], ['public_key' => $public, 'build' => 's7', 'class' => 'A', 'hostname' => 'x1']);
        $code = (string) self::json($r)['deck']['code'];
        if ($member === null) return [null, $code];
        self::assertSame(200, $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return [['Authorization' => 'Bearer ' . self::json($t)['token']], $code];
    }

    /** @return array{0: string, 1: string} the stored chain + key */
    private function store(): array
    {
        [$pem, $key] = LanCertTest::pair('*.lan.scutoons.com');
        $r = $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], ['fullchain' => $pem, 'privkey' => $key]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        return [$pem, $key];
    }

    public function testTheWorkflowStoresTheCertificate(): void
    {
        [$pem, $key] = LanCertTest::pair('*.lan.scutoons.com');
        $body = ['fullchain' => $pem, 'privkey' => $key];
        self::assertSame(403, $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => 'wrong'], $body)->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/deploy/lan-cert', [], $body)->getStatusCode());
        $r = $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], $body);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['*.lan.scutoons.com'], self::json($r)['names']);
        $r = $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], ['fullchain' => $pem]);
        self::assertSame([422, 'invalid_body'], [$r->getStatusCode(), self::json($r)['error']]);
        // without a deploy key the upload does not exist
        self::assertSame(404, $this->request('PUT', '/api/deploy/lan-cert', ['X-Migrate-Key' => self::KEY], $body, ['MIGRATE_KEY' => ''])->getStatusCode());
    }

    public function testAnApprovedDeckFetchesItWithItsOwnToken(): void
    {
        [$member] = $this->session([], 'reader@scutoons.test');
        [$deck] = $this->deck($member);
        self::assertNotNull($deck);
        $r = $this->request('GET', '/api/decks/lan-cert', $deck);
        self::assertSame([404, 'no_certificate'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());

        [$pem, $key] = $this->store();
        $r = $this->request('GET', '/api/decks/lan-cert', $deck);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['lan.scutoons.com', trim($key)], [self::json($r)['domain'], trim((string) self::json($r)['privkey'])]);
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        // deck-tls: one PEM text, the chain, then the key
        $r = $this->request('GET', '/api/decks/lan-cert?format=pem', $deck);
        self::assertSame(['application/x-pem-file', 'no-store'], [$r->getHeaderLine('Content-Type'), $r->getHeaderLine('Cache-Control')]);
        self::assertSame(trim($pem) . "\n" . trim($key) . "\n", (string) $r->getBody());
        // a deck still only reads: it cannot store a certificate
        self::assertSame(403, $this->request('PUT', '/api/deploy/lan-cert', $deck, ['fullchain' => $pem, 'privkey' => $key])->getStatusCode());
    }

    public function testNoAdminTokensAndNoUnpairedDecks(): void
    {
        $this->store();
        [$admin, $user] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        // an admin's AI connection (Phase A's deck-tls) and the admin's own session: refused
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'deck-tls on the X1')['token']];
        foreach ([$ai, $admin] as $headers) {
            $r = $this->request('GET', '/api/decks/lan-cert?format=pem', $headers);
            self::assertSame([403, 'deck_only'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        }
        // the old admin route is gone
        self::assertSame(404, $this->request('GET', '/api/admin/lan-cert', $admin)->getStatusCode());
        // no token, a deck that waits for its approval (it has no token), an unknown deck token
        self::assertSame(401, $this->request('GET', '/api/decks/lan-cert')->getStatusCode());
        [$none] = $this->deck(null);
        self::assertNull($none);
        self::assertSame(401, $this->request('GET', '/api/decks/lan-cert', ['Authorization' => 'Bearer scd_' . str_repeat('a', 40)])->getStatusCode());
        // revoked: 401 at once
        [$member] = $this->session([], 'reader@scutoons.test');
        [$deck] = $this->deck($member);
        self::assertNotNull($deck);
        self::assertSame(200, $this->request('GET', '/api/decks/lan-cert', $deck)->getStatusCode());
        $id = (string) self::json($this->request('GET', '/api/me/decks', $member))['items'][0]['id'];
        self::assertSame(200, $this->request('DELETE', "/api/me/decks/$id", $member)->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/decks/lan-cert', $deck)->getStatusCode());
    }
}
