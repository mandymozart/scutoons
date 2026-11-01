<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogAdmin;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Rigs\RigValidator;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Support\Ids;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;
use stdClass;

/**
 * /api/rigs – members' presets and rig variants over HTTP (#270, agents/research/INSTRUMENTS.md §2, §4–6): deriving
 * keeps only what differs and needs the parent's `derive` right for how it is held; a paired deck writes only with the
 * `rigs` scope its member granted at approval; scene-content serves the member's own presets with `presets=` to their
 * session or deck token – and to nobody else.
 */
final class RigsApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body, JSON_PRESERVE_ZERO_FRACTION));
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

    /** @return array{0: array<string, string>, 1: string} the session header and the user id */
    private function member(string $email): array
    {
        $user = $this->user($email);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    /**
     * A deck paired by $member with $scopes (DecksApiTest's handshake in short) → its bearer header
     * @param array<string, string> $member
     * @param list<string> $scopes
     * @return array<string, string>
     */
    private function deck(array $member, array $scopes): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $secret = ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair);
        $code = (string) self::json($this->request('POST', '/api/decks/request', [], ['public_key' => $public, 'hostname' => 'x1']))['deck']['code'];
        $r = $this->request('POST', '/api/me/decks/approve', $member, ['code' => $code, 'scopes' => $scopes]);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame($scopes, self::json($r)['item']['scopes'], 'the item names what was granted');
        $challenge = (string) self::json($this->request('POST', '/api/decks/challenge', [], ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, $secret));
        $t = $this->request('POST', '/api/decks/token', [], ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }

    public function testDerivingKeepsOnlyWhatChangedAndListsDeletes(): void
    {
        [$me] = $this->member('performer@scutoons.test');
        // cathedral_voice.bright = base + { voice.ratio: 3.0 }; the knobs as turned: the ratio untouched, the index moved
        $r = $this->request('POST', '/api/rigs/presets', $me, ['parent' => 'cathedral_voice.bright', 'name' => ['en' => 'Vespers'],
            'values' => ['voice.ratio' => 3.0, 'voice.index' => 4.0, 'voice.gain' => 0.16]]);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $preset = self::json($r)['preset'];
        self::assertMatchesRegularExpression(RigValidator::MEMBER_PRESET_ID, $preset['id']);
        self::assertSame(['voice.index' => 4.0, 'voice.gain' => 0.16], $preset['values'], 'the ratio equals the parent\'s folded value – not stored');
        self::assertSame(['cathedral_voice', 'cathedral_voice.bright', ['en' => 'Vespers'], ['cathedral', 'happy']], [$preset['instrument'], $preset['parent'], $preset['name'], $preset['tags']], 'the parent\'s tags come along');
        self::assertStringContainsString('"voice.index":4.0', (string) $r->getBody(), 'floats stay floats');

        // a child of the own preset: against the whole folded lineage (base → bright → mine)
        $child = self::json($this->request('POST', '/api/rigs/presets', $me, ['parent' => $preset['id'], 'values' => ['voice.index' => 4.0, 'voice.ratio' => 2.0], 'tags' => ['mine']]))['preset'];
        self::assertSame([['voice.ratio' => 2.0], ['mine'], $preset['id']], [$child['values'], $child['tags'], $child['parent']]);

        $list = self::json($this->request('GET', '/api/rigs/presets', $me));
        self::assertEqualsCanonicalizing([$preset['id'], $child['id']], array_column($list['items'], 'id'), 'both, in creation order (ULIDs made within one millisecond may sort either way)');

        // refused: an unknown field, a value on a linked input, no parent, another member's preset
        foreach ([
            [['parent' => 'cathedral_voice.base', 'color' => 'red'], 422, 'invalid_preset'],
            [['parent' => 'cathedral_voice.base', 'values' => ['melody.cv' => 1.0]], 422, 'invalid_preset'],
            [['values' => ['voice.index' => 1.0]], 422, 'invalid_preset'],
            [['parent' => 'nobody.base'], 422, 'invalid_preset'],
        ] as [$body, $status, $code]) {
            $bad = $this->request('POST', '/api/rigs/presets', $me, $body);
            self::assertSame([$status, $code], [$bad->getStatusCode(), self::json($bad)['error']], (string) json_encode($body));
        }
        [$other] = $this->member('other@scutoons.test');
        self::assertSame(422, $this->request('POST', '/api/rigs/presets', $other, ['parent' => $preset['id']])->getStatusCode(), 'another member\'s preset is unknown to me');
        self::assertSame([], self::json($this->request('GET', '/api/rigs/presets', $other))['items']);
        self::assertSame(404, $this->request('DELETE', "/api/rigs/presets/{$preset['id']}", $other)->getStatusCode());

        // deleting: the parent is in use by its child
        $inUse = $this->request('DELETE', "/api/rigs/presets/{$preset['id']}", $me);
        self::assertSame([409, 'in_use', ["preset {$child['id']}"]], [$inUse->getStatusCode(), self::json($inUse)['error'], self::json($inUse)['uses']]);
        self::assertSame(200, $this->request('DELETE', "/api/rigs/presets/{$child['id']}", $me)->getStatusCode());
        self::assertSame(200, $this->request('DELETE', "/api/rigs/presets/{$preset['id']}", $me)->getStatusCode());
        self::assertSame([], self::json($this->request('GET', '/api/rigs/presets', $me))['items']);
        self::assertSame(401, $this->request('GET', '/api/rigs/presets')->getStatusCode());
    }

    public function testDeriveNeedsTheParentsRightForHowItIsHeld(): void
    {
        [$me, $id] = $this->member('holder@scutoons.test');
        // a factory preset whose definition keeps `derive` for owners: held core (the catalog) it is refused …
        $admin = $this->get(CatalogAdmin::class);
        $locked = json_decode('{"id":"cathedral_voice.locked","instrument":"cathedral_voice","parent":"cathedral_voice.base","values":{"voice.index":5.0},"rights":{"core":["play","tweak"],"owned":["play","tweak","derive"]}}');
        self::assertInstanceOf(stdClass::class, $locked);
        $admin->put('presets', 'cathedral_voice.locked', $locked, 'admin:boss@example.com');
        $r = $this->request('POST', '/api/rigs/presets', $me, ['parent' => 'cathedral_voice.locked', 'values' => ['voice.index' => 6.0]]);
        self::assertSame([403, 'not_derivable', 'core'], [$r->getStatusCode(), self::json($r)['error'], self::json($r)['hold']]);
        // … until the member holds an instrument item of its instrument (the carrier bag: owned)
        $card = json_decode('{"id":"voice_card","name":{"en":"Voice"},"kind":"virtual","category":"instrument","instrument":"cathedral_voice"}');
        self::assertInstanceOf(stdClass::class, $card);
        $admin->put('items', 'voice_card', $card, 'admin:boss@example.com');
        $this->db->run("INSERT INTO inventory (id, user_id, item_id, qty, source, acquired_at, updated_at) VALUES (?, ?, 'voice_card', 1, 'story', ?, ?)", [Ids::uuid(), $id, $this->clock->utc(), $this->clock->utc()]);
        $r = $this->request('POST', '/api/rigs/presets', $me, ['parent' => 'cathedral_voice.locked', 'values' => ['voice.index' => 6.0]]);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame(['voice.index' => 6.0], self::json($r)['preset']['values']);
        self::assertEquals($locked->rights, json_decode((string) json_encode(self::json($r)['preset']['rights'])), 'the derivative carries its parent\'s rights');
    }

    public function testRigVariantsPerSubject(): void
    {
        [$me] = $this->member('rigger@scutoons.test');
        $r = $this->request('GET', '/api/rigs/character.catpope', $me);
        self::assertSame(['success', 'character.catpope', null], array_values(self::json($r)), 'no variant yet');
        self::assertSame(404, $this->request('GET', '/api/rigs/character.nobody', $me)->getStatusCode());
        self::assertSame(404, $this->request('GET', '/api/rigs/Character.x', $me)->getStatusCode(), 'not a subject: no such route');

        $variant = ['track' => [['id' => 'voice', 'instrument' => 'cathedral_voice', 'preset' => 'cathedral_voice.low'], ['id' => 'drone', 'instrument' => 'drone', 'gain' => 0.5]]];
        $r = $this->request('PUT', '/api/rigs/character.catpope', $me, $variant);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        self::assertSame('character.catpope', self::json($r)['rig']['subject'], 'the subject is filled in');
        self::assertSame('cathedral_voice.low', self::json($this->request('GET', '/api/rigs/character.catpope', $me))['rig']['track'][0]['preset']);
        self::assertSame(400, $this->request('PUT', '/api/rigs/character.catpope', $me, $variant + ['subject' => 'character.other'])->getStatusCode());
        $bad = $this->request('PUT', '/api/rigs/character.catpope', $me, ['track' => [['id' => 'air', 'instrument' => 'incense']]]);
        self::assertSame([422, 'invalid_rig'], [$bad->getStatusCode(), self::json($bad)['error']], 'an instrument not held');
        self::assertSame(404, $this->request('PUT', '/api/rigs/character.nobody', $me, $variant)->getStatusCode());
        self::assertSame(200, $this->request('DELETE', '/api/rigs/character.catpope', $me)->getStatusCode());
        self::assertNull(self::json($this->request('GET', '/api/rigs/character.catpope', $me))['rig']);
    }

    public function testADeckWritesOnlyWithTheRigsScopeAndReadsItsOwnPresetsFromSceneContent(): void
    {
        [$me] = $this->member('deckowner@scutoons.test');
        $readOnly = $this->deck($me, []);
        $r = $this->request('POST', '/api/rigs/presets', $readOnly, ['parent' => 'cathedral_voice.base', 'values' => ['voice.index' => 3.0]]);
        self::assertSame([403, 'not_for_deck'], [$r->getStatusCode(), self::json($r)['error']], 'a deck paired without the scope only reads');
        self::assertSame(200, $this->request('GET', '/api/rigs/presets', $readOnly)->getStatusCode());

        $deck = $this->deck($me, ['rigs']);
        $r = $this->request('POST', '/api/rigs/presets', $deck, ['parent' => 'cathedral_voice.base', 'name' => ['en' => 'From the X1'], 'values' => ['voice.index' => 3.0]]);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $mine = self::json($r)['preset']['id'];
        self::assertSame(200, $this->request('PUT', '/api/rigs/character.catpope', $deck, ['track' => [['id' => 'voice', 'instrument' => 'cathedral_voice', 'preset' => $mine]]])->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/me', $deck, ['hide_sensitive' => false])->getStatusCode(), 'the scope opens /api/rigs only');
        self::assertSame(400, $this->request('POST', '/api/me/decks/approve', $me, ['code' => 'ABCDEF', 'scopes' => ['admin']])->getStatusCode(), 'an unknown scope');

        // scene-content: the member's own preset with its ancestor – for the deck and the session; missing for others
        $q = "/api/library/scene-content?items=incense_burner&presets=$mine,cathedral_voice.low,m:01ARZ3NDEKTSV4RRFFQ69G5FAV";
        foreach ([$deck, $me] as $who) {
            $r = $this->request('GET', $q, $who);
            self::assertSame([200, 'private, no-store'], [$r->getStatusCode(), $r->getHeaderLine('Cache-Control')]);
            $a = self::json($r);
            self::assertSame(['cathedral_voice.base', 'cathedral_voice.low', 'incense.base', $mine], array_keys($a['presets']), 'the asked presets and their ancestors, with the rig\'s');
            self::assertSame(['cathedral_voice', 'incense'], array_keys($a['instruments']), 'their instruments too');
            self::assertSame(['en' => 'From the X1'], $a['presets'][$mine]['data']['name']);
            self::assertSame(['m:01ARZ3NDEKTSV4RRFFQ69G5FAV'], $a['missing']['presets']);
            self::assertSame(['incense.base'], $a['items']['incense_burner']['rig']['presets'], 'the factory rig stays the factory rig');
        }
        $anon = self::json($this->request('GET', $q));
        self::assertSame([$mine, 'm:01ARZ3NDEKTSV4RRFFQ69G5FAV'], $anon['missing']['presets'], 'nobody else sees a member\'s preset');
        [$other] = $this->member('stranger@scutoons.test');
        self::assertContains($mine, self::json($this->request('GET', $q, $other))['missing']['presets']);
        $public = $this->request('GET', '/api/library/scene-content?presets=cathedral_voice.low,nobody.base');
        self::assertSame(['public, max-age=60', ['cathedral_voice.base', 'cathedral_voice.low'], ['nobody.base']],
            [$public->getHeaderLine('Cache-Control'), array_keys(self::json($public)['presets']), self::json($public)['missing']['presets']], 'factory presets stay public');
        self::assertSame(400, $this->request('GET', '/api/library/scene-content?presets=M:nope')->getStatusCode());
    }
}
