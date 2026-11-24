<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use ParagonIE_Sodium_Compat;
use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Domain\Library\ExtendedLibrary;
use Scutoons\Domain\Library\UniverseIndex;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Generated\LibraryTypes;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * GET /api/library (#864): things and files in one list (server/schema/api/library.schema.json) – the entry's shape, a
 * thing stepped into, the places, the rights as reading each thing today (a deck token with its member's), the version
 * (ETag, 304) and the pages.
 */
final class ExtendedLibraryApiTest extends TestCase
{
    private const ENTRY_KEYS = ['address', 'type', 'subtype', 'kind', 'name', 'picture', 'parent', 'toons', 'usedIn', 'updated', 'origin'];

    public function testTheListOfAToon(): void
    {
        $this->universe();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $r = $this->request('/api/library?toon=catpope&limit=5000', $creator);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $list = self::json($r);
        self::assertSame([1, 'catpope', 'toon', 'en'], [$list['library'], $list['toon'], $list['place'], $list['locale']]);
        self::assertSame('"' . $list['version'] . '"', $r->getHeaderLine('ETag'));
        self::assertSame('private, no-cache', $r->getHeaderLine('Cache-Control'));
        foreach ($list['entries'] as $e) {
            self::assertSame([], array_diff(array_keys($e), self::ENTRY_KEYS), $e['address']);
            foreach (['address', 'name', 'picture', 'toons', 'usedIn', 'updated'] as $required) self::assertArrayHasKey($required, $e, $e['address']);
            self::assertTrue(isset($e['type']) xor isset($e['kind']), "a thing or a file: {$e['address']}");
            self::assertContains('catpope', $e['toons'], "This toon: {$e['address']}");
        }
        $by = array_column($list['entries'], null, 'address');
        // things first in the type order, then files in the kind order
        $ranks = array_map(fn(array $e) => isset($e['type']) ? array_search($e['type'], LibraryTypes::THING_TYPE, true) : 100 + (int) array_search($e['kind'], LibraryTypes::FILE_KIND, true), $list['entries']);
        $sorted = $ranks;
        sort($sorted);
        self::assertSame($sorted, $ranks);
        self::assertSame(['toon:catpope', 'character:boy', 'character:catpope'], array_slice(array_keys($by), 0, 3));
        self::assertArrayNotHasKey('character:mouse', $by, 'another toon\'s');
        self::assertArrayNotHasKey('character:ghost', $by, 'removed');
        self::assertArrayNotHasKey('toon:toptoy', $by);
        // a character: its name in the locale order, its default look's picture – the smallest rendition of its avatar
        self::assertSame(['address' => 'character:catpope', 'type' => 'character', 'name' => 'Cat Pope', 'picture' => 'https://cdn.test/characters/catpope/catpope_avatar-neutral.small.webp',
            'toons' => ['catpope'], 'usedIn' => 3, 'updated' => '2026-08-01T00:00:00+00:00'], $by['character:catpope']);
        self::assertNull($by['character:boy']['picture'], 'no still yet: null, never a flag');
        self::assertSame('https://cdn.test/characters/catpope/catpope_avatar-neutral.small.webp', $by['toon:catpope']['picture'], 'the toon\'s tile, resolved from its folder');
        // items with their subtype; the shared ones are not this toon's
        self::assertSame(['item', null], [$by['item:camera']['type'], $by['item:camera']['subtype'] ?? null]);
        self::assertSame(['item', 'product'], [$by['product:dago_hoodie']['type'], $by['product:dago_hoodie']['subtype']]);
        self::assertArrayNotHasKey('item:scutoons_tshirt', $by);
        // a rig belongs to its character's toons and names it as its parent; a place by its toons; the scene by its toons
        self::assertSame(['rig', 'character:catpope', ['catpope']], [$by['rig:character.catpope']['type'], $by['rig:character.catpope']['parent'], $by['rig:character.catpope']['toons']]);
        self::assertSame('Dome', $by['place:dom']['name']);
        self::assertSame(['scene', 'Nave'], [$by['scene:nave']['type'], $by['scene:nave']['name']]);
        self::assertArrayNotHasKey('show:duet', $by, 'shows are the live role\'s');
        // files: kind, name, picture, origin, toons from what uses them and from the folder
        self::assertSame(['address' => 'image:characters/catpope/catpope_avatar-neutral.webp', 'kind' => 'image', 'name' => 'catpope_avatar-neutral',
            'picture' => 'https://cdn.test/characters/catpope/catpope_avatar-neutral.small.webp', 'toons' => ['catpope'], 'usedIn' => 2, 'updated' => '2026-08-01T00:00:00+00:00', 'origin' => 'uploaded'],
            $by['image:characters/catpope/catpope_avatar-neutral.webp']);
        self::assertSame(['model', null], [$by['model:catpope/3d/torso.glb']['kind'], $by['model:catpope/3d/torso.glb']['picture']]);
        self::assertArrayNotHasKey('image:catpope/map/world/0/0_0.webp', $by, 'map tiles are never picked one by one');
        self::assertSame(['character' => 2, 'image' => 2, 'item' => 20, 'kanon' => 1, 'model' => 1, 'person' => 1, 'place' => 1, 'rig' => 1, 'scene' => 1, 'toon' => 1], $list['counts']);
        self::assertSame(array_sum($list['counts']), $list['total']);

        // the type and kind filters (the counts stay the place's); a subtype; a locale
        $characters = self::json($this->request('/api/library?toon=catpope&type=character', $creator));
        self::assertSame(['character:boy', 'character:catpope'], array_column($characters['entries'], 'address'));
        self::assertSame($list['counts'], $characters['counts']);
        $mixed = self::json($this->request('/api/library?toon=catpope&type=scene&kind=model', $creator));
        self::assertSame(['scene:nave', 'model:catpope/3d/torso.glb'], array_column($mixed['entries'], 'address'));
        $products = self::json($this->request('/api/library?toon=catpope&type=item&subtype=product', $creator))['entries'];
        self::assertSame(['product'], array_values(array_unique(array_column($products, 'subtype'))));
        self::assertContains('product:dago_hoodie', array_column($products, 'address'));
        $german = self::json($this->request('/api/library?toon=catpope&type=character,place&locale=de', $creator));
        self::assertSame(['Junge', 'Katzenpapst', 'Dom'], array_column($german['entries'], 'name'));

        // the universe: every toon, the shared things, the other toon's character
        $all = array_column(self::json($this->request('/api/library?limit=5000', $creator))['entries'], null, 'address');
        foreach (['toon:toptoy', 'character:mouse', 'item:scutoons_tshirt', 'instrument:cathedral_voice', 'person:mandy'] as $a) self::assertArrayHasKey($a, $all, $a);
        self::assertSame(['toptoy'], $all['character:mouse']['toons']);
        self::assertSame([], $all['instrument:cathedral_voice']['toons'], 'shared');
        self::assertSame(['catpope'], $all['person:mandy']['toons'], 'credited in the toon');
    }

    public function testSteppingIntoACharacter(): void
    {
        $this->universe();
        [$both] = $this->session([Roles::CREATOR, Roles::LIVE], 'both@scutoons.test');
        $r = $this->request('/api/library?at=character:catpope', $both);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        $step = self::json($r);
        self::assertSame([1, 'character:catpope', 'character'], [$step['library'], $step['at']['address'], $step['at']['type']]);
        // personas › looks › moods, parents first; a mood keeps its look's address and names itself
        self::assertSame([
            ['character:catpope.regular', 'persona', 'regular', 'character:catpope', null],
            ['character:catpope.regular.default', 'look', 'default', 'character:catpope.regular', null],
            ['character:catpope.regular.default', 'mood', 'neutral', 'character:catpope.regular.default', 'neutral'],
            ['character:catpope.regular.default', 'mood', 'happy', 'character:catpope.regular.default', 'happy'],
            ['character:catpope.human', 'persona', 'Human', 'character:catpope', null],
            ['character:catpope.human.helmet', 'look', 'helmet', 'character:catpope.human', null],
        ], array_map(fn(array $l) => [$l['address'], $l['level'], $l['name'], $l['parent'], $l['mood'] ?? null], $step['levels']));
        self::assertSame('https://cdn.test/characters/catpope/catpope_avatar-neutral.small.webp', $step['levels'][0]['picture']);
        self::assertSame('https://cdn.test/characters/catpope/catpope_avatar-happy.webp', $step['levels'][3]['picture'], 'a file the registry has no rendition of: its own');
        self::assertNull($step['levels'][4]['picture']);
        // the files it uses, the things it uses, the things that use it – each a link
        self::assertSame(['image:characters/catpope/catpope_avatar-neutral.webp'], array_column($step['files'], 'address'));
        self::assertSame(['rig:character.catpope'], array_column($step['uses'], 'address'));
        self::assertSame([
            ['season:catpope:1', 'Season one', 'cast', null],
            ['chapter:catpope:1', 'The beginning', 'reference', null],
            ['scene:nave', 'Nave', 'reference', 'scene'],
            ['show:duet', 'duet', 'reference', 'show'],
        ], array_map(fn(array $l) => [$l['address'], $l['name'], $l['as'], $l['type'] ?? null], $step['usedIn']));
        self::assertSame(4, $step['at']['usedIn']);

        // a toon: its seasons, then each season's chapters
        $toon = self::json($this->request('/api/library?at=toon:catpope', $both));
        self::assertSame([['season:catpope:1', 'season', 'toon:catpope'], ['chapter:catpope:1', 'chapter', 'season:catpope:1'], ['chapter:catpope:2', 'chapter', 'toon:catpope']],
            array_map(fn(array $l) => [$l['address'], $l['level'], $l['parent']], $toon['levels']));
        self::assertSame(['person:mandy'], array_column($toon['uses'], 'address'), 'credited');
        // an instrument: used in a rig; a file: the documents that use it
        $voice = self::json($this->request('/api/library?at=instrument:cathedral_voice', $both));
        self::assertSame([['rig:character.catpope', 'rig']], array_map(fn(array $l) => [$l['address'], $l['as']], $voice['usedIn']));
        $file = self::json($this->request('/api/library?at=image:characters/catpope/catpope_avatar-neutral.webp', $both));
        self::assertSame(['image', [], [], []], [$file['at']['kind'], $file['levels'], $file['files'], $file['uses']]);
        self::assertSame(['character:catpope', 'toon:catpope'], array_column($file['usedIn'], 'address'));
        self::assertSame(404, $this->request('/api/library?at=character:nobody', $both)->getStatusCode());
    }

    public function testThePlaces(): void
    {
        $this->universe();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $addresses = fn(string $q) => array_column(self::json($this->request("/api/library?$q", $creator))['entries'], 'address');
        // This item: what the thing uses – its things and its files
        self::assertSame(['rig:character.catpope', 'image:characters/catpope/catpope_avatar-neutral.webp'], $addresses('place=item:character:catpope'));
        // a season's cast: its characters and their files
        self::assertSame(['character:boy', 'character:catpope', 'image:characters/catpope/catpope_avatar-neutral.webp'], $addresses('toon=catpope&place=cast:1'));
        self::assertSame(404, $this->request('/api/library?toon=catpope&place=cast:9', $creator)->getStatusCode());
        // Recently added: changed in the last RECENT_DAYS
        self::assertSame(['character:boy', 'image:catpope/fresh.webp'], $addresses('toon=catpope&place=recent'));
        // Unused: files nothing uses
        self::assertSame(['image:catpope/fresh.webp', 'model:catpope/3d/torso.glb'], $addresses('toon=catpope&place=unused'));
        // From: where a file came from (files only)
        self::assertSame(['image:characters/catpope/catpope_avatar-neutral.webp', 'model:catpope/3d/torso.glb'], $addresses('toon=catpope&from=uploaded'));
        self::assertSame(['image:catpope/fresh.webp'], $addresses('toon=catpope&from=generated'));
        // Everything of a toon's view is the universe
        self::assertContains('character:mouse', $addresses('toon=catpope&place=all&limit=5000'));
        // what never answers
        foreach (['place=toon' => 'invalid_place', 'toon=catpope&place=to-do' => 'invalid_place', 'toon=catpope&place=wanted' => 'invalid_place', 'type=bible' => 'invalid_type', 'kind=sound' => 'invalid_kind',
            'from=stolen' => 'invalid_from', 'limit=0' => 'invalid_limit', 'cursor=nope' => 'invalid_cursor', 'locale=deu' => 'invalid_locale', 'toon=Cat!' => 'invalid_toon'] as $q => $error) {
            $r = $this->request("/api/library?$q", $creator);
            self::assertSame([400, $error], [$r->getStatusCode(), self::json($r)['error'] ?? null], $q);
        }
        $unknown = $this->request('/api/library?toon=nope', $creator);
        self::assertSame([404, 'unknown_toon'], [$unknown->getStatusCode(), self::json($unknown)['error'] ?? null]);
    }

    public function testWhoReadsWhat(): void
    {
        $this->universe();
        $path = '/api/library?toon=catpope&limit=5000';
        self::assertSame(401, $this->request($path)->getStatusCode());
        [$member] = $this->session([], 'reader@scutoons.test');
        self::assertSame([403, 'forbidden'], [$this->request($path, $member)->getStatusCode(), self::json($this->request($path, $member))['error'] ?? null]);
        // a deck reads with its member's roles: a member without any reads nothing
        self::assertSame(403, $this->request($path, $this->deck($member))->getStatusCode());

        // the live role: the things the team reads and the shows – no files, no people, no Kanon, no generators
        [$live] = $this->session([Roles::LIVE], 'live@scutoons.test');
        $seen = self::json($this->request('/api/library?limit=5000', $live));
        $kinds = array_values(array_unique(array_map(fn(array $e) => $e['type'] ?? $e['kind'], $seen['entries'])));
        self::assertContains('show', $kinds);
        foreach (['image', 'model', 'person', 'kanon', 'generator'] as $no) self::assertNotContains($no, $kinds, $no);
        self::assertSame(404, $this->request('/api/library?at=image:characters/catpope/catpope_avatar-neutral.webp', $live)->getStatusCode());
        $step = self::json($this->request('/api/library?at=character:catpope', $live));
        self::assertSame([], $step['files'], 'no files for the live role');
        self::assertContains('show:duet', array_column($step['usedIn'], 'address'));

        // a content editor without live: no shows, nor as "used in"
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $mine = self::json($this->request('/api/library?limit=5000', $creator));
        self::assertNotContains('show:duet', array_column($mine['entries'], 'address'));
        self::assertSame(404, $this->request('/api/library?at=show:duet', $creator)->getStatusCode());
        $step = self::json($this->request('/api/library?at=character:catpope', $creator));
        self::assertNotContains('show:duet', array_column($step['usedIn'], 'address'));
        self::assertSame(3, $step['at']['usedIn'], 'the show is not counted either');
        self::assertContains('kanon:character.catpope:de', array_column($mine['entries'], 'address'));

        // the creator's deck reads exactly what the creator reads
        $deck = $this->deck($creator);
        $viaDeck = $this->request('/api/library?limit=5000', $deck);
        self::assertSame(200, $viaDeck->getStatusCode());
        self::assertSame($mine['entries'], self::json($viaDeck)['entries']);
        self::assertSame($mine['version'], self::json($viaDeck)['version'], 'the same rights, the same version');
    }

    public function testTheVersion(): void
    {
        $this->universe();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $first = $this->request('/api/library?toon=catpope', $creator);
        $etag = $first->getHeaderLine('ETag');
        $again = $this->request('/api/library?toon=catpope', $creator, ['If-None-Match' => $etag]);
        self::assertSame([304, '', $etag], [$again->getStatusCode(), (string) $again->getBody(), $again->getHeaderLine('ETag')]);
        self::assertSame(304, $this->request('/api/library?toon=catpope', $creator, ['If-None-Match' => "W/$etag"])->getStatusCode());
        self::assertNotSame($etag, $this->request('/api/library?toon=catpope&locale=de', $creator)->getHeaderLine('ETag'), 'another query, another version');
        // a saved character is a new version, with the change in it
        $this->store('character:boy', ['name' => ['en' => 'Lad'], 'color' => 'green', 'personas' => []], false);
        $changed = $this->request('/api/library?toon=catpope', $creator, ['If-None-Match' => $etag]);
        self::assertSame(200, $changed->getStatusCode());
        self::assertSame('Lad', array_column(self::json($changed)['entries'], 'name', 'address')['character:boy']);
        // an uploaded file too
        $etag = $changed->getHeaderLine('ETag');
        $this->asset('catpope/new.webp', 'image', [['catpope/new.webp', 'image/webp', 100, 'uploaded']]);
        $uploaded = $this->request('/api/library?toon=catpope', $creator, ['If-None-Match' => $etag]);
        self::assertSame(200, $uploaded->getStatusCode());
        self::assertContains('image:catpope/new.webp', array_column(self::json($uploaded)['entries'], 'address'));
    }

    public function testPages(): void
    {
        $this->universe();
        [$creator] = $this->session([Roles::CREATOR], 'creator@scutoons.test');
        $whole = self::json($this->request('/api/library?limit=5000', $creator));
        self::assertNull($whole['next']);
        $seen = [];
        $cursor = null;
        for ($pages = 0; $pages < 50; $pages++) {
            $page = self::json($this->request('/api/library?limit=7' . ($cursor !== null ? "&cursor=$cursor" : ''), $creator));
            self::assertLessThanOrEqual(7, count($page['entries']));
            self::assertSame($whole['total'], $page['total']);
            array_push($seen, ...array_column($page['entries'], 'address'));
            $cursor = $page['next'];
            if ($cursor === null) break;
        }
        self::assertSame(array_column($whole['entries'], 'address'), $seen, 'every entry once, in order');
        self::assertGreaterThan(2, $pages);
    }

    /** the file cases every reader holds to (server/schema/api/library.vectors.json) */
    public function testTheSharedFileCases(): void
    {
        /** @var array{cdn: string, recent_days: int, cases: list<array{what: string, asset: array<string, mixed>, renditions: list<array<string, mixed>>, expect: array<string, mixed>}>} $vectors */
        $vectors = json_decode((string) file_get_contents(self::root() . '/schema/api/library.vectors.json'), true);
        self::assertSame($vectors['recent_days'], ExtendedLibrary::RECENT_DAYS);
        foreach ($vectors['cases'] as $case) {
            $file = UniverseIndex::file($case['asset'] + ['updated_at' => null], $case['renditions'], $vectors['cdn']);
            self::assertSame($case['expect'], array_intersect_key($file, $case['expect']), $case['what']);
        }
    }

    /**
     * Two toons; characters (a cast, an own toon, a removed one), a place, a scene and a show naming characters, chapters, a
     * season, credits and people, a Kanon text, a rig with an instrument; files (one with a smaller rendition, a model, a
     * map tile, a fresh one); the usage index rebuilt. Everything older than the recent window but one character and one file.
     */
    private function universe(): void
    {
        $this->store('toon:catpope', ['id' => 'catpope', 'sourceLanguage' => 'en', 'languages' => ['de', 'en'], 'title' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope'],
            'tile' => ['image' => '../characters/catpope/catpope_avatar-neutral.webp']]);
        $this->store('toon:toptoy', ['id' => 'toptoy', 'title' => ['de' => 'Top Toy']]);
        $this->store('cast:catpope', ['seasons' => ['1' => [['character' => 'catpope', 'personas' => ['regular']], ['character' => 'boy', 'personas' => ['regular']]]]]);
        $this->store('character:catpope', ['name' => ['de' => 'Katzenpapst', 'en' => 'Cat Pope'], 'color' => 'blue', 'personas' => [
            ['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => [
                'neutral' => ['avatar' => 'characters/catpope/catpope_avatar-neutral.webp'],
                'happy' => ['avatar' => 'characters/catpope/catpope_avatar-happy.webp'],
            ]]]],
            ['id' => 'human', 'label' => ['en' => 'Human'], 'looks' => [['id' => 'helmet']]],
        ]]);
        $this->store('character:ghost', ['removed' => true, 'name' => ['en' => 'Ghost'], 'color' => 'white', 'personas' => []]);
        $this->store('character:mouse', ['name' => ['en' => 'Mouse'], 'color' => 'yellow', 'own_toon' => 'toptoy', 'personas' => []]);
        $this->store('place:dom', ['name' => ['de' => 'Dom', 'en' => 'Dome'], 'toons' => ['catpope'], 'x' => 1, 'y' => 2]);
        $this->store('scene:nave', ['format' => 2, 'id' => 'nave', 'version' => 1, 'toons' => ['catpope'], 'holder' => [['id' => 'cp', 'ref' => 'character:catpope.regular', 'rig' => ['ref' => 'rig:character.catpope']]],
            'stage' => ['name' => 'Nave'], 'node' => [], 'link' => []]);
        $this->store('show:duet', ['id' => 'duet', 'toons' => ['catpope'], 'rows' => [['id' => 'a', 'cells' => ['left' => ['piece' => 'scene:nave', 'who' => 'character:catpope']]]]], false);
        $this->store('season:catpope:1', ['number' => 1, 'title' => ['en' => 'Season one'], 'sequence' => [['number' => 1, 'status' => 'published']]]);
        $this->store('chapter:catpope:1', ['number' => 1, 'title' => ['en' => 'The beginning'], 'groups' => [['id' => 'a', 'events' => [['type' => 'show', 'params' => ['character' => 'catpope']], ['type' => 'giveItem', 'params' => ['item' => 'camera']]]]]]);
        $this->store('chapter:catpope:2', ['number' => 2, 'title' => ['en' => 'Later'], 'groups' => []]);
        $this->store('people', ['people' => [['id' => 'mandy', 'name' => 'Mandy Mozart'], ['id' => 'una', 'name' => 'Una Ryu']]]);
        $this->store('credits:catpope', ['toon' => 'catpope', 'sections' => [['id' => 'toon', 'lead' => [['person' => 'mandy', 'roles' => [['role' => 'writer']]]]]]]);
        $this->store('kanon:character.catpope:de', ['id' => 'character.catpope', 'text' => 'Ein Papst.'], false);
        // the catalog the migrations seed has the rig `character.catpope` (instruments cathedral_voice, drone), modules, recipes …
        $this->asset('characters/catpope/catpope_avatar-neutral.webp', 'image', [
            ['characters/catpope/catpope_avatar-neutral.webp', 'image/webp', 1170, 'uploaded'],
            ['characters/catpope/catpope_avatar-neutral.small.webp', 'image/webp', 128, 'generated'],
        ]);
        $this->asset('catpope/3d/torso.glb', 'model', [['catpope/3d/torso.glb', 'model/gltf-binary', null, 'uploaded']]);
        $this->asset('catpope/map/world/0/0_0.webp', 'image', [['catpope/map/world/0/0_0.webp', 'image/webp', 256, 'imported']]);
        $old = '2026-08-01 00:00:00';
        $this->db->run('UPDATE entity_versions SET created_at = ?', [$old]);
        $this->db->run('UPDATE assets SET updated_at = ?', [$old]);
        foreach (['items', 'recipes', 'products', 'editions', 'albums', 'home-scenes', 'modules', 'instruments', 'rigs'] as $kind) {
            $this->db->run('UPDATE ' . CatalogStore::table($kind) . ' SET updated_at = ?', [$old]);
        }
        // the fresh ones: a character saved today, a file uploaded today
        $this->store('character:boy', ['name' => ['de' => 'Junge', 'en' => 'Boy'], 'color' => 'green', 'personas' => []], false);
        $this->asset('catpope/fresh.webp', 'image', [['catpope/fresh.webp', 'image/webp', 300, 'generated']]);
        $this->get(AssetUsages::class)->rebuild();
    }

    /** @param list<array{0: string, 1: string, 2: ?int, 3: string}> $files the main file first: path, mime, width, origin */
    private function asset(string $main, string $kind, array $files): void
    {
        $registry = $this->get(AssetRegistry::class);
        $id = $registry->create(preg_replace('/\.[a-z0-9]+$/', '', $main) ?? $main, $kind);
        $version = $registry->addVersion($id, new StoredFile($main, 10, '', null, $files[0][1]), 'local', ['width' => $files[0][2]]);
        foreach ($files as [$path, $mime, $width, $origin]) {
            $registry->addRendition($id, $version, $path === $main ? 'source' : 'small', null, new StoredFile($path, 10, '', null, $mime), ['width' => $width], $origin);
        }
    }

    /** @param array<mixed> $doc */
    private function store(string $key, array $doc, bool $publish = true): void
    {
        $store = $this->get(EntityStore::class);
        $json = (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        if ($publish) $store->insertPublished(EntityKey::parse($key), $json, null);
        else $store->insert(EntityKey::parse($key), $json, null, 'draft', null);
    }

    /** @return array<string, string> */
    protected function settings(): array
    {
        return ['CDN_URL' => 'https://cdn.test/'] + parent::settings();
    }

    /**
     * @param array<string, string> $headers
     * @param array<string, string> $more
     */
    private function request(string $uri, array $headers = [], array $more = [], string $method = 'GET', mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        $request = $request->withQueryParams((function () use ($uri): array {
            parse_str((string) parse_url($uri, PHP_URL_QUERY), $q);
            return $q;
        })());
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
     * A deck paired by $member (the handshake, as AddressBookApiTest) → its bearer header
     * @param array<string, string> $session the pairing member's session header
     * @return array<string, string>
     */
    private function deck(array $session): array
    {
        $pair = ParagonIE_Sodium_Compat::crypto_sign_keypair();
        $public = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_publickey($pair));
        $r = $this->request('/api/decks/request', [], [], 'POST', ['public_key' => $public, 'build' => 's7', 'class' => 'web', 'hostname' => 'browser']);
        $code = (string) self::json($r)['deck']['code'];
        self::assertSame(200, $this->request('/api/me/decks/approve', $session, [], 'POST', ['code' => $code])->getStatusCode());
        $challenge = (string) self::json($this->request('/api/decks/challenge', [], [], 'POST', ['public_key' => $public]))['deck']['challenge'];
        $signature = bin2hex(ParagonIE_Sodium_Compat::crypto_sign_detached(Decks::SIGNED_PREFIX . $challenge . ':' . $public, ParagonIE_Sodium_Compat::crypto_sign_secretkey($pair)));
        $t = $this->request('/api/decks/token', [], [], 'POST', ['public_key' => $public, 'challenge' => $challenge, 'signature' => $signature]);
        self::assertSame(200, $t->getStatusCode(), (string) $t->getBody());
        return ['Authorization' => 'Bearer ' . self::json($t)['token']];
    }
}
