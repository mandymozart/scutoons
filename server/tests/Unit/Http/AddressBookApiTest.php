<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Generated\AddressBookTypes;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * GET /api/addresses/<toon> (#477): every addressable thing of a toon's catalog in the generated shape
 * (server/schema/api/address-book.schema.json) – read by the team (creator, admin, live) and approved decks, versioned by
 * the catalog it was built from (ETag, 304).
 */
final class AddressBookApiTest extends TestCase
{
    public function testTheDocument(): void
    {
        $this->catalog();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $r = $this->request('/api/addresses/catpope', $creator);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $book = self::json($r);
        self::assertSame(AddressBookTypes::FORMAT, $book['address_book']);
        self::assertSame(['catpope', 'en'], [$book['toon'], $book['source']], 'the toon\'s source language');
        self::assertSame('"' . $book['version'] . '"', $r->getHeaderLine('ETag'));
        self::assertMatchesRegularExpression('/"labels":\{\}/', (string) $r->getBody(), 'an empty label map stays an object');
        $by = array_column($book['entries'], null, 'ref');
        // every key of the shape, nothing else (the generated PHP shape is what PHPStan checks; this is the wire)
        foreach ($book['entries'] as $e) {
            self::assertSame([], array_diff(array_keys($e), ['ref', 'address', 'kind', 'name', 'parent', 'labels', 'origin', 'moods']), $e['ref']);
            self::assertContains($e['kind'], AddressBookTypes::ADDRESS_KIND);
            self::assertContains($e['origin']['state'], AddressBookTypes::ORIGIN_STATE);
            // an address only where a receiver of the kind exists, and then the reference's text (decisions/deck.md [address])
            $addressed = in_array($e['kind'], ['character', 'persona', 'look', 'item'], true);
            self::assertSame($addressed ? $e['ref'] : null, $e['address'] ?? null, $e['ref']);
        }
        // characters: the cast (season order), then own_toon; the removed one and the other toon's are not in it
        $characters = array_values(array_filter($book['entries'], fn(array $e) => $e['kind'] === 'character'));
        self::assertSame(['character:catpope', 'character:boy', 'character:crow'], array_column($characters, 'ref'));
        self::assertSame(['address' => 'character:catpope', 'kind' => 'character', 'name' => 'catpope', 'labels' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope'], 'origin' => ['key' => 'character:catpope', 'state' => 'published']],
            array_diff_key($by['character:catpope'], ['ref' => 1]));
        self::assertSame(['ref' => 'character:catpope.human', 'address' => 'character:catpope.human', 'kind' => 'persona', 'name' => 'human', 'parent' => 'character:catpope', 'labels' => ['de' => 'Mensch', 'en' => 'human']],
            array_diff_key($by['character:catpope.human'], ['origin' => 1]));
        self::assertSame(['ref' => 'character:catpope.human.helmet', 'address' => 'character:catpope.human.helmet', 'kind' => 'look', 'name' => 'helmet', 'parent' => 'character:catpope.human', 'moods' => ['neutral', 'happy']],
            array_diff_key($by['character:catpope.human.helmet'], ['origin' => 1, 'labels' => 1]));
        self::assertSame('draft', $by['character:boy']['origin']['state'], 'a newer draft: the draft is the entry');
        self::assertSame(['de' => 'Junge (neu)'], $by['character:boy']['labels']);
        self::assertArrayNotHasKey('character:ghost', $by, 'removed');
        self::assertArrayNotHasKey('character:mouse', $by, 'another toon\'s');
        // locations: the places whose toons hold it
        self::assertSame(['location:dom'], array_column(array_filter($book['entries'], fn(array $e) => $e['kind'] === 'location'), 'ref'));
        self::assertSame(['key' => 'place:dom', 'state' => 'published'], $by['location:dom']['origin']);
        // items: the toon's and the shared ones (the fixture catalog: catpope's and no toon's)
        self::assertSame(['de' => 'Kamera', 'en' => 'Camera'], $by['item:camera']['labels']);
        self::assertArrayHasKey('item:scutoons_tshirt', $by, 'a shared item');
        // chapters by number, each followed by its groups (a label once; a label that is no address word gets a safe form), then
        // the variables of the one save game
        $chapters = array_values(array_filter($book['entries'], fn(array $e) => in_array($e['kind'], ['chapter', 'group'], true)));
        self::assertSame(['chapter:2', 'chapter:2.intro', 'chapter:2.Das-Ende', 'chapter:2.Das-Ende-2', 'chapter:10'], array_column($chapters, 'ref'));
        self::assertSame(['2', ['de' => 'Zwei']], [$by['chapter:2']['name'], $by['chapter:2']['labels']]);
        self::assertSame(['kind' => 'group', 'name' => 'Das Ende?', 'parent' => 'chapter:2', 'labels' => [], 'origin' => ['key' => 'chapter:catpope:2', 'state' => 'published']],
            array_diff_key($by['chapter:2.Das-Ende-2'], ['ref' => 1]), 'the name is the label as the event stores it');
        $variables = array_values(array_filter($book['entries'], fn(array $e) => $e['kind'] === 'variable'));
        self::assertSame(array_fill(0, count($variables), 'variable'), array_column(array_slice($book['entries'], -count($variables)), 'kind'), 'variables last');
        self::assertSame(['variable:catpope.coins' => 'coins', 'variable:global.stars' => 'global.stars', 'variable:toptoy.luck' => 'toptoy.luck'],
            array_column($variables, 'name', 'ref'), 'as written in this toon');
        self::assertSame(['key' => 'toon:toptoy', 'state' => 'published'], $by['variable:toptoy.luck']['origin']);

        // the other toon: only the shared items, its own character; the same variables, as written there
        $other = self::json($this->request('/api/addresses/toptoy', $creator));
        self::assertSame(['character:mouse'], array_column(array_filter($other['entries'], fn(array $e) => $e['kind'] === 'character'), 'ref'));
        self::assertNotContains('item:camera', array_column($other['entries'], 'ref'));
        $otherVariables = array_column(array_filter($other['entries'], fn(array $e) => $e['kind'] === 'variable'), 'name', 'ref');
        self::assertSame(['catpope.coins', 'luck'], [$otherVariables['variable:catpope.coins'], $otherVariables['variable:toptoy.luck']]);
    }

    public function testTheVersionIsTheCatalogs(): void
    {
        $this->catalog();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $first = $this->request('/api/addresses/catpope', $creator);
        $etag = $first->getHeaderLine('ETag');
        $again = $this->request('/api/addresses/catpope', $creator, ['If-None-Match' => $etag]);
        self::assertSame([304, '', $etag], [$again->getStatusCode(), (string) $again->getBody(), $again->getHeaderLine('ETag')]);
        self::assertSame($etag, $this->request('/api/addresses/catpope', $creator)->getHeaderLine('ETag'), 'unchanged: the same version');
        // a saved character changes it, and the new document has the change
        $this->store('character:catpope', ['name' => ['de' => 'Papst'], 'color' => 'red', 'personas' => []], false);
        $changed = $this->request('/api/addresses/catpope', $creator, ['If-None-Match' => $etag]);
        self::assertSame(200, $changed->getStatusCode());
        self::assertNotSame($etag, $changed->getHeaderLine('ETag'));
        $by = array_column(self::json($changed)['entries'], null, 'ref');
        self::assertSame(['de' => 'Papst'], $by['character:catpope']['labels']);
        self::assertArrayNotHasKey('character:catpope.human', $by);
        // a variable another toon sets (one save game) changes it too
        $before = $changed->getHeaderLine('ETag');
        $this->store('chapter:toptoy:2', ['number' => 2, 'groups' => [['label' => 'a', 'events' => [['type' => 'setVariable', 'params' => ['variable' => 'global.moons', 'value' => '1']]]]]], false);
        $moons = $this->request('/api/addresses/catpope', $creator, ['If-None-Match' => $before]);
        self::assertSame(200, $moons->getStatusCode());
        self::assertContains('variable:global.moons', array_column(self::json($moons)['entries'], 'ref'));
    }

    public function testWhoReadsIt(): void
    {
        $this->catalog();
        $path = '/api/addresses/catpope';
        self::assertSame(401, $this->request($path)->getStatusCode());
        [$member] = $this->session([], 'reader@scutoons.test');
        self::assertSame([403, 'forbidden'], [$this->request($path, $member)->getStatusCode(), self::json($this->request($path, $member))['error'] ?? null]);
        foreach ([Roles::CREATOR, Roles::ADMIN, Roles::LIVE] as $role) {
            [$headers] = $this->session([$role], "$role@scutoons.test");
            self::assertSame(200, $this->request($path, $headers)->getStatusCode(), $role);
        }
        // an approved deck reads it with its own token (its member needs no role, as for show media)
        $deck = $this->deck($member);
        self::assertSame(200, $this->request($path, $deck)->getStatusCode());
        // an AI connection acts with its member's roles (only creators and the like have one)
        [$creator, $creatorUser] = $this->session([Roles::CREATOR], 'writer@scutoons.test');
        $ai = ['Authorization' => 'Bearer ' . $this->get(ApiTokens::class)->create($creatorUser, 'a script')['token']];
        self::assertSame(200, $this->request($path, $ai)->getStatusCode());
        self::assertSame([404, 'unknown_toon'], [$this->request('/api/addresses/nope', $creator)->getStatusCode(), self::json($this->request('/api/addresses/nope', $creator))['error'] ?? null]);
    }

    /** A small catalog: two toons, characters (cast, own toon, a draft, a removed one), places, chapters */
    private function catalog(): void
    {
        $look = fn(string $id, array $moods, bool $default = false) => ['id' => $id] + ($default ? ['default' => true] : [])
            + ['images' => array_fill_keys($moods, ['portrait' => "p/$id.webp"])];
        $this->store('toon:catpope', ['id' => 'catpope', 'sourceLanguage' => 'en', 'languages' => ['de', 'en'], 'title' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope']]);
        $this->store('toon:toptoy', ['id' => 'toptoy', 'title' => ['de' => 'Top Toy']]);
        $this->store('cast:catpope', ['seasons' => ['2' => [['character' => 'boy', 'personas' => ['regular']]], '1' => [['character' => 'catpope', 'personas' => ['regular', 'human']], ['character' => 'ghost', 'personas' => ['regular']]]]]);
        $this->store('character:catpope', ['name' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope'], 'color' => 'blue', 'personas' => [
            ['id' => 'regular', 'looks' => [$look('default', ['neutral'], true)]],
            ['id' => 'human', 'label' => ['de' => 'Mensch', 'en' => 'human'], 'looks' => [$look('default', ['neutral'], true), $look('helmet', ['neutral', 'happy'])]],
        ]]);
        $this->store('character:boy', ['name' => ['de' => 'Junge'], 'color' => 'green', 'personas' => [['id' => 'regular', 'looks' => [$look('default', [])]]]]);
        $this->store('character:boy', ['name' => ['de' => 'Junge (neu)'], 'color' => 'green', 'personas' => [['id' => 'regular', 'looks' => [$look('default', [])]]]], false);
        $this->store('character:ghost', ['removed' => true, 'name' => ['de' => 'Geist'], 'color' => 'white', 'personas' => []]);
        $this->store('character:crow', ['name' => ['de' => 'Krähe'], 'color' => 'black', 'own_toon' => 'catpope', 'personas' => []]);
        $this->store('character:mouse', ['name' => ['de' => 'Maus'], 'color' => 'yellow', 'own_toon' => 'toptoy', 'personas' => []]);
        $this->store('place:dom', ['name' => ['de' => 'Dom'], 'toons' => ['catpope'], 'x' => 1, 'y' => 2]);
        $this->store('place:shop', ['name' => ['de' => 'Laden'], 'toons' => ['toptoy'], 'x' => 1, 'y' => 2]);
        $this->store('place:gone', ['removed' => true, 'name' => ['de' => 'Weg'], 'toons' => ['catpope'], 'x' => 1, 'y' => 2]);
        $this->store('chapter:catpope:10', ['number' => 10, 'title' => ['de' => 'Zehn'], 'groups' => []]);
        $set = fn(string $variable) => ['type' => 'setVariable', 'params' => ['variable' => $variable, 'value' => '1']];
        $this->store('chapter:catpope:2', ['number' => 2, 'title' => ['de' => 'Zwei'], 'groups' => [
            ['label' => 'intro', 'events' => [$set('coins')]],
            ['label' => 'Das Ende.'],
            ['label' => 'Das Ende?'],
            ['label' => 'intro'],
            ['events' => [$set('global.stars')]],
        ]]);
        $this->store('chapter:toptoy:1', ['number' => 1, 'title' => ['de' => 'Eins'], 'groups' => [['label' => 'start', 'events' => [$set('luck')]]]]);
    }

    /** @param array<mixed> $doc */
    private function store(string $key, array $doc, bool $publish = true): void
    {
        $store = $this->get(EntityStore::class);
        $json = (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($publish) $store->insertPublished(EntityKey::parse($key), $json, null);
        else $store->insert(EntityKey::parse($key), $json, null, 'draft', null);
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $more
     */
    private function request(string $uri, array $headers = [], array $more = [], string $method = 'GET', mixed $body = null): ResponseInterface
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
     * A deck paired by $member (the handshake, as DeckMediaApiTest) → its bearer header
     * @param array<string, string> $member
     * @return array<string, string>
     */
    private function deck(array $member): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $r = $this->request('/api/decks/request', [], [], 'POST', ['public_key' => $public, 'build' => 's7', 'class' => 'web', 'hostname' => 'browser']);
        $code = (string) self::json($r)['deck']['code'];
        self::assertSame(200, $this->request('/api/me/decks/approve', $member, [], 'POST', ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->request('/api/decks/challenge', [], [], 'POST', ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->request('/api/decks/token', [], [], 'POST', ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }
}
