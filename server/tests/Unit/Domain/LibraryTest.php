<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\ContentFiles;
use Scutoons\Console\ImportContentCommand;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Library\Library;
use Scutoons\Tests\TestCase;

/** The library service (agents/DATA.md §3): composition, locale fallback, visibility, draft mode, cache keys */
final class LibraryTest extends TestCase
{
    private bool $showDrafts = false;

    protected function settings(): array
    {
        return ['CONTENT_SHOW_DRAFTS' => $this->showDrafts ? '1' : '0'] + parent::settings();
    }

    /**
     * Stores a version of the entity a file of the interchange layout stands for (published unless told otherwise);
     * `<toon>/_/index.json` = the toon's index entry, `scutoons/_/events.json` = the events
     */
    private function doc(string $toon, string $locale, string $path, mixed $data, bool $publish = true): int
    {
        $key = match (true) {
            $path === 'index.json' => "toon:$toon",
            $path === 'events.json' => 'events',
            default => ContentFiles::keyOf($toon, $locale, $path),
        };
        self::assertNotNull($key, "$toon/$locale/$path");
        $store = $this->get(EntityStore::class);
        $json = json_encode($data, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
        $id = $store->insert(EntityKey::parse($key), (string) $json, null, 'draft', null);
        if ($publish) $store->setPublished(['id' => $id, 'entity_key' => $key]);
        return $id;
    }

    private function library(): Library
    {
        return $this->get(Library::class);
    }

    /** @return array<mixed> */
    private static function decode(string $body): array
    {
        return (array) json_decode($body, true);
    }

    /** A toon `t` (source de, languages de + en): season 1 = ch. 1 published, 2 draft, 3 coming-soon; ch. 1 has a scene */
    private function seedToon(): void
    {
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en']]);
        foreach (['de', 'en'] as $l) {
            $this->doc('t', $l, 'toon.json', ['id' => 't', 'title' => "T $l", 'status' => 'published']);
            $this->doc('t', $l, 'season1.json', ['title' => "S $l", 'sequence' => [
                ['type' => 'chapter', 'number' => 1, 'status' => 'published'],
                ['type' => 'chapter', 'number' => 2, 'status' => 'draft'],
                ['type' => 'chapter', 'number' => 3, 'status' => 'coming-soon'],
            ]]);
            $this->doc('t', $l, 'chapter1.json', ['title' => "Eins $l", 'number' => 1, 'release' => '2025-10-04', 'thumbnail' => 'x.webp', 'status' => 'published',
                'audio' => ['volume' => 1.0], 'groups' => [['ref' => ['src' => 'chapter1/scene-1.json']], ['label' => 'inline', 'slices' => []], ['ref' => ['src' => 'chapter1/gone.json']]]]);
            $this->doc('t', $l, 'chapter1/scene-1.json', ['label' => "scene $l", 'events' => [['type' => 'joinScene', 'params' => ['character' => 'plf']]], 'params' => new \stdClass()]);
            $this->doc('t', $l, 'chapter2.json', ['title' => "Zwei $l", 'number' => 2, 'groups' => []]);
            $this->doc('t', $l, 'locations.json', [['id' => "loc-$l"]]);
        }
        $this->doc('t', 'de', 'chapter3.json', ['title' => 'Drei', 'number' => 3, 'release' => '2026-12-01', 'groups' => [['label' => 'secret']]]);
    }

    public function testToonBundleWithSeasonOverview(): void
    {
        $this->seedToon();
        $answer = $this->library()->toon('t', 'en', false);
        $data = self::decode($answer->body);
        self::assertSame('en', $answer->locale);
        self::assertTrue($answer->fallback, 'chapter 3 exists in de only');
        self::assertSame(['entry', 'toon', 'seasons', 'locations', 'routes'], array_keys($data));
        self::assertSame('T en', $data['toon']['title']);
        self::assertSame([['id' => 'loc-en']], $data['locations']);
        self::assertSame([], $data['routes'], 'missing lists are empty');
        $chapters = $data['seasons'][0]['chapters'];
        self::assertSame([1, 3], array_column($chapters, 'number'), 'the draft chapter is not listed, coming-soon is');
        self::assertSame(['title' => 'Eins en', 'number' => 1, 'release' => '2025-10-04', 'thumbnail' => 'x.webp', 'type' => 'chapter', 'status' => 'published'], $chapters[0],
            'summary = chapter without groups/audio/own status + season type/number/status');
        self::assertSame('Drei', $chapters[1]['title'], 'coming-soon: en file missing → de file');
        self::assertArrayNotHasKey('groups', $chapters[1]);
    }

    public function testChapterWithScenesAndLocaleFallback(): void
    {
        $this->seedToon();
        $answer = $this->library()->chapter('t', 1, 'en', false);
        $data = self::decode($answer->body);
        self::assertSame(['en', false], [$answer->locale, $answer->fallback]);
        self::assertSame('scene en', $data['groups'][0]['label'], 'ref group replaced by the scene file');
        self::assertSame('inline', $data['groups'][1]['label']);
        self::assertSame(['ref' => ['src' => 'chapter1/gone.json']], $data['groups'][2], 'a missing scene keeps its ref group');
        self::assertSame(['scene:t:1:gone'], $answer->missing);
        self::assertStringContainsString('"params":{}', $answer->body, '{} stays an object');
        self::assertStringContainsString('"volume":1.0', $answer->body, 'zero fractions are kept');

        $fr = $this->library()->chapter('t', 1, 'fr', false);
        self::assertSame(['de', true], [$fr->locale, $fr->fallback], 'unknown locale → source language, flagged');
        self::assertSame('Eins de', self::decode($fr->body)['title']);
    }

    public function testChapterGates(): void
    {
        $this->seedToon();
        foreach ([[2, 'not_public'], [3, 'coming_soon'], [9, 'not_public']] as [$number, $code]) {
            try {
                $this->library()->chapter('t', $number, 'de', false);
                self::fail("chapter $number: $code expected");
            } catch (ApiError $error) {
                self::assertSame([403, $code], [$error->status, $error->errorCode], "chapter $number");
                if ($code === 'coming_soon') {
                    self::assertSame(['title' => 'Drei', 'number' => 3, 'release' => '2026-12-01', 'type' => 'chapter', 'status' => 'coming-soon'], json_decode((string) json_encode($error->extra['chapter']), true));
                }
            }
        }
        // draft mode (creators): no gates, the newest version
        $this->doc('t', 'de', 'chapter2.json', ['title' => 'Zwei neu', 'number' => 2, 'groups' => []], false);
        self::assertSame('Zwei neu', self::decode($this->library()->chapter('t', 2, 'de', true)->body)['title']);
        self::assertSame('Drei', self::decode($this->library()->chapter('t', 3, 'de', true)->body)['title']);
    }

    public function testShowDraftsSetting(): void
    {
        $this->showDrafts = true;
        $this->setUp();
        $this->seedToon();
        self::assertSame('Zwei de', self::decode($this->library()->chapter('t', 2, 'de', false)->body)['title']);
        self::assertSame([1, 2, 3], array_column(self::decode($this->library()->toon('t', 'de', false)->body)['seasons'][0]['chapters'], 'number'));
        try {
            $this->library()->chapter('t', 3, 'de', false);
            self::fail('coming soon stays gated');
        } catch (ApiError $error) {
            self::assertSame('coming_soon', $error->errorCode);
        }
    }

    public function testToonVisibilityAndList(): void
    {
        $this->seedToon();
        $this->doc('d', '_', 'index.json', ['id' => 'd', 'status' => 'draft']);
        $this->doc('d', 'de', 'toon.json', ['id' => 'd', 'status' => 'draft']);
        $this->doc('u', 'de', 'toon.json', ['id' => 'u']); // no index entry
        $this->doc('z', '_', 'index.json', ['id' => 'z', 'status' => 'published'], false); // never published
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(false)->body), 'id'));
        self::assertSame(['t', 'd', 'z'], array_column(self::decode($this->library()->toons(true)->body), 'id'), 'draft mode: every entry, creation order');
        foreach (['d', 'u', 'nope', Library::PLATFORM] as $toon) {
            try {
                $this->library()->toon($toon, null, false);
                self::fail("$toon: 404 expected");
            } catch (ApiError $error) {
                self::assertSame(404, $error->status, $toon);
            }
        }
        self::assertSame('d', self::decode($this->library()->toon('d', null, true)->body)['toon']['id'], 'creators see draft toons');
        try {
            $this->library()->toon('Bad!', null, false);
            self::fail('invalid toon');
        } catch (ApiError $error) {
            self::assertSame('invalid_toon', $error->errorCode);
        }
    }

    /** Announced toons (owner 2026-10-01): listed publicly as "soon" with their tile, their pages stay 404; never the sandbox */
    public function testAnnouncedToonsAndTilesInTheList(): void
    {
        $this->seedToon();
        $tile = ['image' => '../characters/t/t_portrait-neutral.webp'];
        $map = ['layers' => ['terrain' => 'm.svg'], 'width' => 2, 'height' => 1, 'center' => ['x' => 1, 'y' => 0]];
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en'], 'tile' => $tile, 'map' => $map]);
        $this->doc('a', '_', 'index.json', ['id' => 'a', 'status' => 'draft', 'sourceLanguage' => 'de', 'announce' => true, 'tile' => ['image' => '../characters/a/a_helmet.webp', 'color' => '#ff0'], 'authors' => ['X']]);
        $this->doc('a', 'de', 'toon.json', ['id' => 'a', 'status' => 'draft']);
        $this->doc('d', '_', 'index.json', ['id' => 'd', 'status' => 'draft', 'announce' => false]);
        $this->doc('d', 'de', 'toon.json', ['id' => 'd', 'status' => 'draft']);
        $this->doc('sandbox', '_', 'index.json', ['id' => 'sandbox', 'status' => 'draft', 'announce' => true]);
        $this->doc('sandbox', 'de', 'toon.json', ['id' => 'sandbox', 'status' => 'draft']);
        $this->doc('n', '_', 'index.json', ['id' => 'n', 'status' => 'draft']);
        $this->doc('n', '_', 'index.json', ['id' => 'n', 'status' => 'draft', 'announce' => true], false); // announce not published yet

        $list = self::decode($this->library()->toons(false)->body);
        self::assertSame(['t', 'a'], array_column($list, 'id'), 'published + announced; not the draft, the sandbox, an unpublished announce');
        self::assertSame($tile, $list[0]['tile']);
        self::assertArrayNotHasKey('map', $list[0], 'the list carries the tile only of the neutral settings');
        self::assertArrayNotHasKey('announced', $list[0], 'published toons are not "announced"');
        self::assertSame(['id' => 'a', 'status' => 'draft', 'sourceLanguage' => 'de', 'announced' => true,
            'tile' => ['image' => '../characters/a/a_helmet.webp', 'color' => '#ff0']], $list[1], 'the entry + announced + tile, nothing else');
        foreach (['a', 'sandbox'] as $toon) {
            try {
                $this->library()->toon($toon, 'de', false);
                self::fail("$toon: 404 expected");
            } catch (ApiError $error) {
                self::assertSame(404, $error->status, "$toon: an announced toon's pages stay closed");
            }
        }
        self::assertSame(['t', 'a', 'd', 'sandbox', 'n'], array_column(self::decode($this->library()->toons(true)->body), 'id'), 'draft mode (creator): every entry');
    }

    /** Older data (before 2026-10-01): `announce` + `tile` still in the toon settings are read from there */
    public function testAnnounceAndTileFromOlderToonSettings(): void
    {
        $this->doc('a', '_', 'index.json', ['id' => 'a', 'status' => 'draft', 'sourceLanguage' => 'de']);
        $this->doc('a', 'de', 'toon.json', ['id' => 'a', 'status' => 'draft', 'announce' => true, 'tile' => ['image' => 'a.webp']]);
        $list = self::decode($this->library()->toons(false)->body);
        self::assertSame([['id' => 'a', 'status' => 'draft', 'sourceLanguage' => 'de', 'announced' => true, 'tile' => ['image' => 'a.webp']]], $list);
    }

    /** The language-neutral settings (index entry) are merged into every locale's `toon`; `entry` stays the plain index entry */
    public function testToonBundleMergesTheNeutralSettings(): void
    {
        $this->seedToon();
        $map = ['layers' => ['terrain' => 'images/map/m.svg'], 'width' => 2048, 'height' => 1046, 'center' => ['x' => 1320, 'y' => 440]];
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en'],
            'authors' => ['A'], 'chapters' => 5, 'thumbnail' => 'th.webp', 'map' => $map, 'tile' => ['image' => 'tile.webp', 'color' => '#0066cc']]);
        // a stale per-language value loses against the index entry
        $this->doc('t', 'en', 'toon.json', ['id' => 't', 'title' => 'T en', 'status' => 'published', 'map' => ['width' => 0]]);
        foreach (['de', 'en'] as $l) {
            $data = self::decode($this->library()->toon('t', $l, false)->body);
            self::assertSame("T $l", $data['toon']['title']);
            self::assertSame($map, $data['toon']['map'], "$l: the map of the index entry");
            self::assertSame(['A'], $data['toon']['authors']);
            self::assertSame(5, $data['toon']['chapters']);
            self::assertSame('th.webp', $data['toon']['thumbnail']);
            self::assertSame(['image' => 'tile.webp', 'color' => '#0066cc'], $data['toon']['tile']);
            self::assertSame(['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en']], $data['entry'], 'entry without the settings');
        }
    }

    public function testShowDraftsListsDraftsAndTheSandbox(): void
    {
        $this->showDrafts = true;
        $this->setUp();
        $this->doc('a', '_', 'index.json', ['id' => 'a', 'status' => 'draft', 'announce' => true]);
        $this->doc('a', 'de', 'toon.json', ['id' => 'a', 'status' => 'draft']);
        $this->doc('sandbox', '_', 'index.json', ['id' => 'sandbox', 'status' => 'draft']);
        $list = self::decode($this->library()->toons(false)->body);
        self::assertSame(['a', 'sandbox'], array_column($list, 'id'), 'local development (CONTENT_SHOW_DRAFTS) keeps listing drafts');
        self::assertSame([true, null], [$list[0]['announced'] ?? null, $list[1]['announced'] ?? null]);
    }

    public function testCastHasTheStarAndTheChapterFeaturing(): void
    {
        $this->seedToon();
        foreach (['de', 'en'] as $l) {
            $this->doc('t', $l, 'chapter2.json', ['title' => "Zwei $l", 'number' => 2, 'groups' => [], 'featuring' => ['star', 'crows']]);
        }
        $this->doc('t', '_', 'cast.json', ['seasons' => ['1' => [['character' => 'boy', 'personas' => ['regular']], ['character' => 'crows', 'personas' => ['regular']]], '2' => [['character' => 'star', 'personas' => ['regular', 'x']]]]]);
        $this->doc(Library::PLATFORM, '_', 'characters.json', ['characters' => [['id' => 'star', 'own_toon' => 't'], ['id' => 'boy'], ['id' => 'crows'], ['id' => 'other', 'own_toon' => 'u']]]);
        $all = self::decode($this->library()->cast('t', null, false)->body);
        self::assertSame(['star', 'boy', 'crows'], array_column($all['cast']['seasons']['1'], 'character'), 'the star is added in front');
        self::assertSame([['character' => 'star', 'personas' => ['regular', 'x']]], $all['cast']['seasons']['2'], 'a season with the star stays as it is');
        self::assertSame(['star', 'boy', 'crows'], array_column($all['registry']['characters'], 'id'));
        $chapter = self::decode($this->library()->cast('t', 2, true)->body);
        self::assertSame(['star', 'crows'], array_column($chapter['registry']['characters'], 'id'), 'a chapter: its featuring');
        $bundle = self::decode($this->library()->toon('t', 'de', true)->body);
        self::assertSame(['star', 'crows'], $bundle['seasons'][0]['chapters'][1]['featuring'], 'the season overview passes featuring through');
    }

    public function testSeasonDownloadAndSupportersPassThrough(): void
    {
        $this->seedToon();
        $download = ['music' => 'audio/album.zip', 'image' => 'images/s1-music-vinyl.webp'];
        $supporters = [['name' => 'A', 'logo' => 'images/supporters/a.svg'], ['name' => 'B', 'url' => 'https://b.example']];
        $trailer = ['loop' => 'videos/l.1280w.mp4', 'sources' => [['src' => 'videos/l.1920w.mp4', 'media' => '(min-width: 1600px)']], 'poster' => 'images/p.webp', 'chapter' => 0];
        foreach (['de', 'en'] as $l) {
            $this->doc('t', $l, 'season1.json', ['title' => "S $l", 'download' => $download, 'supporters' => $supporters, 'trailer' => $trailer, 'sequence' => [['type' => 'chapter', 'number' => 1, 'status' => 'published']]]);
        }
        $bundle = self::decode($this->library()->toon('t', 'en', false)->body);
        self::assertSame($download, $bundle['seasons'][0]['season']['download'], 'the season download (the music album) is passed through');
        self::assertSame($supporters, $bundle['seasons'][0]['season']['supporters'], 'the supporters are passed through in order');
        self::assertSame($trailer, $bundle['seasons'][0]['season']['trailer'], 'the trailer (desktop hero loop) is passed through');
        $season = self::decode($this->library()->season('t', 1, 'de', false)->body);
        self::assertSame($supporters, $season['season']['supporters']);
    }

    public function testCastAndCredits(): void
    {
        $this->seedToon();
        $this->doc('t', '_', 'cast.json', ['seasons' => ['1' => [['character' => 'boy', 'personas' => ['regular']]]]]);
        $this->doc(Library::PLATFORM, '_', 'characters.json', ['characters' => [['id' => 'catpope'], ['id' => 'boy'], ['id' => 'crows']]]);
        $all = self::decode($this->library()->cast('t', null, false)->body);
        self::assertSame(['boy'], array_column($all['registry']['characters'], 'id'), 'reduced to the cast');
        self::assertSame('boy', $all['cast']['seasons']['1'][0]['character']);
        $chapter = self::decode($this->library()->cast('t', 1, false)->body);
        self::assertSame(['catpope'], array_column($chapter['registry']['characters'], 'id'), 'a chapter: the characters it references (legacy plf → catpope)');

        $this->doc('t', '_', 'credits.json', ['toon' => 't', 'sections' => [['kind' => 'toon', 'lead' => [['person' => 'old', 'roles' => []]], 'entries' => [['person' => 'b', 'roles' => []]]]]]);
        $this->doc(Library::PLATFORM, '_', 'people.json', ['people' => [['id' => 'a'], ['id' => 'b'], ['id' => 'old', 'merged_into' => 'new'], ['id' => 'new'], ['id' => 'x']]]);
        $this->doc(Library::PLATFORM, '_', 'roles.json', ['roles' => [['id' => 'writer']]]);
        $credits = self::decode($this->library()->credits('t', false)->body);
        self::assertSame(['b', 'old', 'new'], array_column($credits['people']['people'], 'id'), 'referenced people + merge targets');
        self::assertSame('writer', $credits['roles']['roles'][0]['id']);
        try {
            $this->library()->credits('nope', false);
            self::fail('404');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
    }

    public function testCacheKeyChangesOnPublishAndDraftsAreNotCached(): void
    {
        $this->seedToon();
        $first = $this->library()->chapter('t', 1, 'de', false);
        self::assertNotEmpty(glob($this->dir . '/cache/library/*.cache'), 'answer cached');
        $again = $this->library()->chapter('t', 1, 'de', false);
        self::assertSame([$first->etag, $first->body, $first->missing], [$again->etag, $again->body, $again->missing], 'from the cache');

        $count = count(glob($this->dir . '/cache/library/*.cache') ?: []);
        $this->library()->chapter('t', 1, 'de', true);
        self::assertCount($count, glob($this->dir . '/cache/library/*.cache') ?: [], 'draft answers are not cached');

        $this->doc('t', 'de', 'chapter1/scene-1.json', ['label' => 'scene de v2'], true);
        $after = $this->library()->chapter('t', 1, 'de', false);
        self::assertNotSame($first->etag, $after->etag, 'a publish changes the key');
        self::assertSame('scene de v2', self::decode($after->body)['groups'][0]['label']);
    }

    public function testEvents(): void
    {
        try {
            $this->library()->events(false);
            self::fail('404 before any events');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
        $this->doc(Library::PLATFORM, '_', 'events.json', [['title' => 'Cat Pope', 'city' => 'Wien']]);
        self::assertSame([['title' => 'Cat Pope', 'city' => 'Wien']], self::decode($this->library()->events(false)->body));
    }

    /** The world map (agents/MAP.md): a published toon `t`, a draft toon `d`, the sandbox; places of each + one of nobody */
    private function seedWorldMap(): void
    {
        $this->seedToon();
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en'], 'map' => ['center' => ['x' => 100, 'y' => 50]]]);
        $this->doc('d', '_', 'index.json', ['id' => 'd', 'uri' => 'd', 'status' => 'draft', 'announce' => true]);
        $this->doc('sandbox', '_', 'index.json', ['id' => 'sandbox', 'uri' => 'sandbox', 'status' => 'draft']);
        $this->doc(Library::PLATFORM, '_', 'map.json', [
            'width' => 2048, 'height' => 1046,
            'layers' => [['id' => 'terrain', 'image' => 't/images/map/terrain.svg']],
            'locations' => [
                ['id' => 'home', 'name' => ['de' => 'Zuhause', 'en' => 'Home'], 'x' => 90, 'y' => 40, 'toons' => ['t'], 'chapter' => ['toon' => 't', 'number' => 1]],
                ['id' => 'tower', 'name' => ['de' => 'Turm'], 'x' => 120, 'y' => 60, 'toons' => ['t'], 'hidden' => true, 'unlock' => ['toon' => 't', 'chapter' => 1, 'at' => 0.5]],
                ['id' => 'stage', 'name' => ['de' => 'Bühne', 'en' => 'Stage'], 'x' => 500, 'y' => 500, 'toons' => ['d']],
                ['id' => 'pond', 'name' => ['de' => 'Teich', 'en' => 'Pond'], 'x' => 700, 'y' => 600, 'toons' => ['sandbox']],
                ['id' => 'nowhere', 'name' => ['de' => 'Nirgends'], 'x' => 1, 'y' => 1, 'toons' => []],
            ],
            'routes' => [
                ['id' => 'way', 'name' => ['de' => 'Weg', 'en' => 'Way'], 'toons' => ['t'], 'style' => 'trail',
                    'waypoints' => [['location' => 'home'], ['id' => 'bridge', 'x' => 105, 'y' => 45, 'name' => ['de' => 'Brücke']], ['location' => 'tower'], ['location' => 'gone']]],
                ['id' => 'secret', 'name' => ['de' => 'Geheim'], 'toons' => ['d'], 'waypoints' => [['location' => 'stage'], ['location' => 'home']]],
            ],
        ]);
    }

    public function testWorldMapForReaders(): void
    {
        $this->seedWorldMap();
        $answer = $this->library()->map('en', null, false);
        $data = self::decode($answer->body);
        self::assertSame([2048, 1046], [$data['width'], $data['height']]);
        self::assertSame('t/images/map/terrain.svg', $data['layers'][0]['image']);
        // published toons only: the draft (announced) toon's, the sandbox's and nobody's places stay out
        self::assertSame(['home', 'tower'], array_column($data['locations'], 'id'));
        self::assertSame(['Home', 'Turm'], array_column($data['locations'], 'name'), 'names resolved, missing en → de');
        self::assertTrue($answer->fallback);
        self::assertSame('en', $answer->locale);
        self::assertTrue($data['locations'][1]['hidden'], 'hidden places are sent (the unlock state is client-side)');
        self::assertSame(['toon' => 't', 'chapter' => 1, 'at' => 0.5], $data['locations'][1]['unlock']);
        self::assertSame(['way'], array_column($data['routes'], 'id'));
        self::assertSame('Brücke', $data['routes'][0]['waypoints'][1]['name'], 'free waypoint names resolved');
        self::assertSame(['location' => 'gone'], $data['routes'][0]['waypoints'][3], 'unknown places stay (the reader skips them)');
        self::assertArrayNotHasKey('view', $data);

        $view = self::decode($this->library()->map('de', 't', false)->body)['view'];
        self::assertSame(['center' => ['x' => 100, 'y' => 50], 'zoom' => 1], $view);
        try {
            $this->library()->map('de', 'd', false);
            self::fail('a draft toon has no public map view');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
        // draft mode (creators): every toon
        self::assertSame(['home', 'tower', 'stage', 'pond'], array_column(self::decode($this->library()->map('de', null, true)->body)['locations'], 'id'));
    }

    public function testWorldMapShowsDraftToonsWithTheSetting(): void
    {
        $this->showDrafts = true;
        $this->setUp();
        $this->seedWorldMap();
        self::assertSame(['home', 'tower', 'stage', 'pond'], array_column(self::decode($this->library()->map('de', null, false)->body)['locations'], 'id'));
    }

    public function testWorldMapMissing404(): void
    {
        try {
            $this->library()->map('de', null, false);
            self::fail('404 without a world map');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
    }

    /** Older viewer builds: the bundle's `locations` / `routes` / `toon.map` in the old shape, computed from the world map */
    public function testToonBundleComputesTheOldMapFromTheWorld(): void
    {
        $this->seedWorldMap();
        $data = self::decode($this->library()->toon('t', 'en', false)->body);
        self::assertSame([
            ['id' => 'home', 'label' => 'Home', 'toonId' => 't', 'type' => 'chapter', 'chapterNumber' => '1', 'image' => '', 'coordinates' => ['x' => 90, 'y' => 40]],
            ['id' => 'tower', 'label' => 'Turm', 'toonId' => 't', 'type' => 'chapter', 'image' => '', 'coordinates' => ['x' => 120, 'y' => 60]],
        ], $data['locations'], 'the old documents (locations:t:*) are no longer read once the world map exists');
        self::assertSame([['id' => 'way', 'title' => 'Way', 'locations' => ['home', 'tower', 'gone']]], $data['routes']);
        self::assertSame(['layers' => ['terrain' => 'images/map/terrain.svg'], 'width' => 2048, 'height' => 1046, 'center' => ['x' => 100, 'y' => 50]], $data['toon']['map']);
        self::assertArrayNotHasKey('map', $data['entry']);
    }

    public function testImportSourcesAndExportRoundTrip(): void
    {
        $root = $this->dir . '/static';
        mkdir("$root/t/de/chapter1", 0777, true);
        file_put_contents("$root/toons.json", json_encode([['id' => 'b', 'status' => 'published'], ['id' => 'a', 'status' => 'draft'], ['id' => 'Bad!']]));
        file_put_contents("$root/events.json", '[{"title":"E"}]');
        foreach (['toon', 'season1', 'chapter1', 'locations', 'routes', 'collectables', 'characters', 'credits'] as $name) {
            file_put_contents("$root/t/de/$name.json", json_encode(['name' => $name]));
        }
        file_put_contents("$root/t/de/chapter1/scene-1.json", '{"params":{}}');
        $keys = array_column(ContentFiles::sources($root), 0);
        self::assertSame(['toon:b', 'toon:a', 'events', 'chapter:t:1:de', 'scene:t:1:scene-1:de',
            'locations:t:de', 'routes:t:de', 'season:t:1:de', 'toon-settings:t:de'], $keys,
            'toons.json split in order, events, the map; legacy characters/credits.json and the removed collectables are not store files');

        $tester = new \Symfony\Component\Console\Tester\CommandTester($this->get(ImportContentCommand::class));
        self::assertSame(0, $tester->execute(['root' => $root]));
        self::assertSame(['b'], array_column(self::decode($this->library()->toons(false)->body), 'id'));

        $out = $this->dir . '/export';
        $export = new \Symfony\Component\Console\Tester\CommandTester($this->get(\Scutoons\Console\ExportContentCommand::class));
        self::assertSame(0, $export->execute(['dir' => $out]));
        self::assertSame([['id' => 'b', 'status' => 'published'], ['id' => 'a', 'status' => 'draft']], json_decode((string) file_get_contents("$out/toons.json"), true));
        self::assertSame([['title' => 'E']], json_decode((string) file_get_contents("$out/events.json"), true));
        self::assertSame('{' . "\n" . '    "params": {}' . "\n}\n", file_get_contents("$out/t/de/chapter1/scene-1.json"));
        self::assertFileDoesNotExist("$out/t/de/characters.json");
        $keysAgain = array_column(ContentFiles::sources($out), 0);
        self::assertSame($keys, $keysAgain, 'the export reads back as the same documents');
    }
}
