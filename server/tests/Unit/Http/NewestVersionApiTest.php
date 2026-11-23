<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * `?only=version` on an item's GET (RULES H19, #858): the newest version's id, `ETag: "<id>"`, 304 on a match – with the
 * read rights of the item's own route, for a member's session and a deck's token alike.
 */
final class NewestVersionApiTest extends TestCase
{
    private const EVENT = ['title' => 'Cat Pope', 'date' => '2026-11-14T20:00:00', 'location' => 'Arena', 'city' => 'Vienna', 'category' => 'irl', 'toon' => ['catpope']];

    public function testTheNewestVersionAndItsTag(): void
    {
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $first = (int) self::json($this->request('PUT', '/api/entities/events', $creator, ['content' => []]))['version']['id'];
        $path = '/api/entities/events?only=version';

        $r = $this->request('GET', $path, $creator);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['status' => 'success', 'key' => 'events', 'version' => $first], self::json($r));
        self::assertSame("\"$first\"", $r->getHeaderLine('ETag'));
        self::assertStringContainsString('ETag', $r->getHeaderLine('Access-Control-Expose-Headers'));

        $same = $this->request('GET', $path, $creator, null, ['If-None-Match' => "\"$first\""]);
        self::assertSame([304, '', "\"$first\""], [$same->getStatusCode(), (string) $same->getBody(), $same->getHeaderLine('ETag')]);
        self::assertSame(304, $this->request('GET', $path, $creator, null, ['If-None-Match' => "\"1\", W/\"$first\""])->getStatusCode(), 'a list, a weak tag');

        // a save (another window's) is a newer version: 200 with its id
        $second = (int) self::json($this->request('PUT', '/api/entities/events', $creator, ['content' => [self::EVENT], 'base' => $first]))['version']['id'];
        $newer = $this->request('GET', $path, $creator, null, ['If-None-Match' => "\"$first\""]);
        self::assertSame([200, $second, "\"$second\""], [$newer->getStatusCode(), self::json($newer)['version'], $newer->getHeaderLine('ETag')]);
        self::assertSame(304, $this->request('GET', $path, $creator, null, ['If-None-Match' => "\"$second\""])->getStatusCode());

        // a publish changes no version id; a restore is a new draft
        self::assertSame(200, $this->request('POST', '/api/entities/events/publish', $creator)->getStatusCode());
        self::assertSame(304, $this->request('GET', $path, $creator, null, ['If-None-Match' => "\"$second\""])->getStatusCode());
        $restored = (int) self::json($this->request('POST', '/api/entities/events/restore', $creator, ['id' => $first]))['version']['id'];
        self::assertSame($restored, self::json($this->request('GET', $path, $creator, null, ['If-None-Match' => "\"$second\""]))['version']);
    }

    public function testWhoAsksAsReadingDoes(): void
    {
        $this->store('events', []);
        $path = '/api/entities/events?only=version';
        self::assertSame(401, $this->request('GET', $path)->getStatusCode());
        [$reader] = $this->session([], 'reader@scutoons.test');
        $no = $this->request('GET', $path, $reader);
        self::assertSame([403, 'forbidden'], [$no->getStatusCode(), self::json($no)['error'] ?? null]);
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $missing = $this->request('GET', '/api/entities/chapter:catpope:99?only=version', $creator);
        self::assertSame([404, 'not_found'], [$missing->getStatusCode(), self::json($missing)['error'] ?? null]);
        self::assertSame(400, $this->request('GET', '/api/entities/bogus:thing?only=version', $creator)->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/entities/show:gala?only=version', $creator)->getStatusCode(), 'a show goes through /api/shows');

        // a deck's token: its member's rights – a reader's deck no more than the reader, a creator's deck as the creator
        self::assertSame(403, $this->request('GET', $path, $this->deck($reader))->getStatusCode());
        $deck = $this->deck($creator);
        $r = $this->request('GET', $path, $deck);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $etag = $r->getHeaderLine('ETag');
        self::assertSame(304, $this->request('GET', $path, $deck, null, ['If-None-Match' => $etag])->getStatusCode());
    }

    public function testAShowForTheLiveRoleAndItsDeck(): void
    {
        $show = ['id' => 'gala', 'outlets' => [['id' => 'left']], 'seats' => [], 'rows' => []];
        [$live] = $this->session([Roles::LIVE], 'live@scutoons.test');
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $saved = $this->request('PUT', '/api/shows/gala', $live, ['content' => $show]);
        self::assertSame(200, $saved->getStatusCode(), (string) $saved->getBody());
        $id = (int) self::json($saved)['version']['id'];
        $path = '/api/shows/gala?only=version';
        self::assertSame(403, $this->request('GET', $path, $creator)->getStatusCode(), 'the live role reads shows, as /api/shows/<id>');
        $deck = $this->deck($live);
        $r = $this->request('GET', $path, $deck);
        self::assertSame([200, 'show:gala', $id, "\"$id\""], [$r->getStatusCode(), self::json($r)['key'] ?? null, self::json($r)['version'] ?? null, $r->getHeaderLine('ETag')]);
        self::assertSame(304, $this->request('GET', $path, $deck, null, ['If-None-Match' => "\"$id\""])->getStatusCode());
        $next = (int) self::json($this->request('PUT', '/api/shows/gala', $live, ['content' => ['rows' => [['cells' => []]]] + $show, 'base' => $id]))['version']['id'];
        self::assertSame($next, self::json($this->request('GET', $path, $deck, null, ['If-None-Match' => "\"$id\""]))['version']);
        self::assertSame(404, $this->request('GET', '/api/shows/none?only=version', $live)->getStatusCode());
    }

    public function testASceneForTheContentEditorsOnly(): void
    {
        $this->store('scene:nave', ['format' => 2, 'id' => 'nave', 'version' => 1, 'toons' => [], 'stage' => ['name' => 'Nave'], 'node' => [], 'link' => []]);
        $path = '/api/scenes/nave?only=version';
        [$reader] = $this->session([], 'reader@scutoons.test');
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        self::assertSame(403, $this->request('GET', $path, $reader)->getStatusCode(), 'the newest of a scene is a draft: editors only');
        $r = $this->request('GET', $path, $creator);
        self::assertSame([200, 'scene:nave'], [$r->getStatusCode(), self::json($r)['key'] ?? null]);
        self::assertSame(304, $this->request('GET', $path, $creator, null, ['If-None-Match' => $r->getHeaderLine('ETag')])->getStatusCode());
    }

    /** @param array<mixed> $doc */
    private function store(string $key, array $doc): void
    {
        $this->get(EntityStore::class)->insert(EntityKey::parse($key), (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE), null, 'draft', null);
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $more
     */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null, array $more = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers + $more as $name => $value) $request = $request->withHeader($name, $value);
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
     * A deck paired by the member (the handshake, as AddressBookApiTest) → its bearer header
     * @param array<string, string> $member
     * @return array<string, string>
     */
    private function deck(array $member): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $code = (string) self::json($this->request('POST', '/api/decks/request', [], ['public_key' => $public, 'build' => 's7', 'class' => 'web', 'hostname' => 'browser']))['deck']['code'];
        self::assertSame(200, $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }
}
