<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\Controllers\AssetsController;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Show media for decks (#470, #484, DecksController::media): a deck with an account gets a show medium, as a short
 * signed link on the vault's pull zone – an approved deck's own token, or the sign-in of a browser deck's user; an AI
 * connection (an admin's too) gets 403, no token / an unknown or revoked deck 401; only `live/<toon>/…` paths – never
 * another vault file (a score's PDF) and never `..`.
 */
final class DeckMediaApiTest extends TestCase
{
    protected function settings(): array
    {
        return ['PROTECTED_CDN_URL' => 'https://vault.example.net/', 'BUNNY_PROTECTED_CDN_KEY' => 'test-key'] + parent::settings();
    }

    public function testAnApprovedDeckGetsASignedLink(): void
    {
        [$member] = $this->session([], 'reader@scutoons.test');
        $deck = $this->deck($member);
        $r = $this->request('GET', '/api/decks/media?path=live/catpope/stems/choir.ogg', $deck);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        $body = self::json($r);
        $expires = $this->clock->time() + 600;
        self::assertSame($expires, $body['expires']);
        // bunny.net token authentication over the path + expires (ProtectedFiles::signed)
        $token = 'HS256-' . rtrim(strtr(base64_encode(hash_hmac('sha256', '/live/catpope/stems/choir.ogg' . $expires, 'test-key', true)), '+/', '-_'), '=');
        self::assertSame("https://vault.example.net/live/catpope/stems/choir.ogg?token=$token&expires=$expires", $body['url']);
    }

    public function testOnlyShowMediaPaths(): void
    {
        [$member] = $this->session([], 'reader@scutoons.test');
        $deck = $this->deck($member);
        foreach (['', 'scores/hymn/abc.pdf', 'albums/a1/t.mp3', 'live/catpope', 'live/catpope/', 'live/catpope/../../scores/x.pdf',
                     'live/catpope/./x.ogg', 'live/Cat/x.ogg', 'live/catpope/.hidden', 'live//x.ogg', '/live/catpope/x.ogg',
                     'live/catpope/a/b/c/d/e/f.ogg', 'live/catpope/x y.ogg'] as $path) {
            $r = $this->request('GET', '/api/decks/media?path=' . rawurlencode($path), $deck);
            self::assertSame([400, 'invalid_path'], [$r->getStatusCode(), self::json($r)['error'] ?? null], $path);
        }
        self::assertSame(400, $this->request('GET', '/api/decks/media', $deck)->getStatusCode(), 'no path');
        self::assertSame(400, $this->request('GET', '/api/decks/media?path[]=live/catpope/x.ogg', $deck)->getStatusCode(), 'not a string');
    }

    public function testASignInIsADeckOfItsUserNoAiConnectionsNoUnpairedDecks(): void
    {
        [$admin, $user] = $this->session([Roles::ADMIN], 'admin@scutoons.test');
        // #484: a browser deck in a signed-in viewer is its user – a member without any role too
        [$member] = $this->session([], 'member@scutoons.test');
        foreach ([$admin, $member] as $headers) {
            $r = $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg', $headers);
            self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
            self::assertStringContainsString('live/catpope/x.ogg', (string) self::json($r)['url']);
        }
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($user, 'a script')['token']];
        $r = $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg', $ai);
        self::assertSame([403, 'deck_only'], [$r->getStatusCode(), self::json($r)['error'] ?? null], (string) $r->getBody());
        self::assertSame(401, $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg')->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg', ['Authorization' => 'Bearer scd_' . str_repeat('a', 40)])->getStatusCode());
        // revoked: 401 at once
        [$member] = $this->session([], 'reader@scutoons.test');
        $deck = $this->deck($member);
        self::assertSame(200, $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg', $deck)->getStatusCode());
        $id = (string) self::json($this->request('GET', '/api/me/decks', $member))['items'][0]['id'];
        self::assertSame(200, $this->request('DELETE', "/api/me/decks/$id", $member)->getStatusCode());
        self::assertSame(401, $this->request('GET', '/api/decks/media?path=live/catpope/x.ogg', $deck)->getStatusCode());
        // a deck still only reads: no upload of show media with a deck token
        $r = $this->request('PUT', '/api/assets/protected?show=catpope&file=media', $this->deck($member), ['x' => 1]);
        self::assertSame([403, 'not_for_deck'], [$r->getStatusCode(), self::json($r)['error'] ?? null]);
    }

    public function testAShowMediumsTypeComesFromItsBytes(): void
    {
        $riff = fn(string $kind) => 'RIFF' . pack('V', 100) . $kind;
        $cases = [
            ["\0\0\0\x20ftypisom", 'mp4'], ["\x1A\x45\xDF\xA3\x9F", 'webm'], ["glTF\x02\0\0\0", 'glb'], ["\x89PNG\r\n\x1A\n", 'png'],
            ["\xFF\xD8\xFF\xE0", 'jpg'], [$riff('WEBP'), 'webp'], [$riff('WAVE'), 'wav'], ['OggS', 'ogg'], ['fLaC', 'flac'],
            ['ID3', 'mp3'], ["\xFF\xFB\x90", 'mp3'],
        ];
        foreach ($cases as [$head, $ext]) self::assertSame($ext, AssetsController::showMediumType($head)[0] ?? null, $ext);
        foreach (['<?php echo 1;', '<svg xmlns="x">', '%PDF-1.7', "PK\x03\x04", $riff('AVI '), ''] as $head) {
            self::assertNull(AssetsController::showMediumType($head), $head);
        }
    }

    /**
     * @param array<string, string> $headers
     */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        $query = [];
        parse_str((string) parse_url("http://localhost$uri", PHP_URL_QUERY), $query);
        $request = $request->withQueryParams($query);
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
     * @return array{array<string, string>, array<string, mixed>} the session header + the user
     */
    private function session(array $roles, string $email): array
    {
        $user = $this->user($email);
        foreach ($roles as $role) $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], $user];
    }

    /**
     * A deck paired by $member (deck-link's handshake, as LanApiTest) → its bearer header
     * @param array<string, string> $member
     * @return array<string, string>
     */
    private function deck(array $member): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $r = $this->request('POST', '/api/decks/request', [], ['public_key' => $public, 'build' => 's7', 'class' => 'web', 'hostname' => 'browser']);
        $code = (string) self::json($r)['deck']['code'];
        self::assertSame(200, $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }
}
