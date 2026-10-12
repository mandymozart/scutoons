<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\ContentFiles;
use Scutoons\Console\ImportContentCommand;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\CollectionDocs;
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
        $collection = CollectionDocs::ofAnchor($key);
        if ($collection !== null) {
            // the world map's / the registry's documents (agents/MAP-DOCUMENTS.md): the anchor + one per entry, like an import
            $first = 0;
            foreach (CollectionDocs::split($collection, json_decode((string) json_encode($data))) as $part => $doc) {
                $id = $store->insert(EntityKey::parse($part), (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION), null, 'draft', null);
                if ($publish) $store->setPublished(['id' => $id, 'entity_key' => $part]);
                $first = $first ?: $id;
            }
            return $first;
        }
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

    /** seedToon()'s toon entry: the toon's settings as locale maps (agents/NEUTRAL.md step 4) + more fields for one test */
    private const ENTRY = ['id' => 't', 'uri' => 't', 'status' => 'published', 'sourceLanguage' => 'de', 'languages' => ['de', 'en']];
    private const ENTRY_TEXTS = ['title' => ['de' => 'T de', 'en' => 'T en'], 'languageStatus' => ['de' => 'published', 'en' => 'published']];
    /** the name in an answered entry: `title` per language + the legacy `i18n` computed from it (decision 4) */
    private const ENTRY_NAME = ['title' => ['de' => 'T de', 'en' => 'T en'], 'i18n' => ['de' => ['name' => 'T de'], 'en' => ['name' => 'T en']]];

    /**
     * A toon `t` (source de, languages de + en): season 1 = ch. 1 published, 2 draft, 3 coming-soon; ch. 1 has a scene.
     * The toon entry holds the toon's settings per language; chapters, scenes and the season are language-neutral
     * documents (texts per locale, agents/NEUTRAL.md); the locations are per locale.
     * @param array<string, mixed> $entry more fields of the toon entry
     */
    private function seedToon(array $entry = []): void
    {
        $this->doc('t', '_', 'index.json', $entry + self::ENTRY + self::ENTRY_TEXTS);
        foreach (['de', 'en'] as $l) $this->doc('t', $l, 'locations.json', [['id' => "loc-$l"]]);
        $this->seedSeason();
        $this->doc('t', '_', 'chapter1.json', ['title' => ['de' => 'Eins de', 'en' => 'Eins en'], 'number' => 1, 'release' => ['de' => '2025-10-04', 'en' => '2025-10-04'], 'thumbnail' => 'x.webp',
            'audio' => ['volume' => 1.0], 'groups' => [['ref' => ['src' => 'chapter1/scene-1.json']], ['label' => 'inline', 'slices' => []], ['ref' => ['src' => 'chapter1/gone.json']]]]);
        $this->doc('t', '_', 'chapter1/scene-1.json', ['label' => 'scene', 'events' => [['type' => 'joinScene', 'params' => ['character' => 'plf']]], 'params' => new \stdClass()]);
        $this->doc('t', '_', 'chapter2.json', ['title' => ['de' => 'Zwei de', 'en' => 'Zwei en'], 'number' => 2, 'groups' => []]);
        $this->doc('t', '_', 'chapter3.json', ['title' => ['de' => 'Drei'], 'number' => 3, 'release' => ['de' => '2026-12-01'], 'groups' => [['label' => 'secret']]]);
    }

    /**
     * seedToon()'s season 1 (language-neutral; more fields for one test)
     * @param array<string, mixed> $more
     */
    private function seedSeason(array $more = []): void
    {
        $both = fn(string $status) => ['de' => $status, 'en' => $status];
        $this->doc('t', '_', 'season1.json', ['title' => ['de' => 'S de', 'en' => 'S en'], 'number' => 1] + $more + ['sequence' => [
            ['number' => 1, 'status' => $both('published')],
            ['number' => 2, 'status' => $both('draft')],
            ['number' => 3, 'status' => $both('coming-soon')],
        ]]);
    }

    /**
     * seedToon() as legacy per-locale documents (production before its conversion, agents/NEUTRAL.md): the library's
     * fallback path when an entity has no neutral document – the toon entry without texts, per-locale toon settings
     */
    private function seedLegacy(): void
    {
        $this->doc('t', '_', 'index.json', self::ENTRY);
        foreach (['de', 'en'] as $l) {
            $this->doc('t', $l, 'toon.json', ['id' => 't', 'title' => "T $l", 'status' => 'published']);
            $this->doc('t', $l, 'season1.json', ['title' => "S $l", 'sequence' => [
                ['number' => 1, 'status' => 'published'],
                ['number' => 2, 'status' => 'draft'],
                ['number' => 3, 'status' => 'coming-soon'],
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
        self::assertSame(['title' => 'Eins en', 'number' => 1, 'release' => '2025-10-04', 'thumbnail' => 'x.webp', 'language' => 'en', 'status' => 'published'], $chapters[0],
            'summary = chapter without groups/audio/own status (+ the served language) + season type/number/status');
        self::assertSame('Drei', $chapters[1]['title'], 'coming-soon: en file missing → de file');
        self::assertArrayNotHasKey('groups', $chapters[1]);
    }

    public function testChapterWithScenesAndLocaleFallback(): void
    {
        $this->seedToon();
        $answer = $this->library()->chapter('t', 1, 'en', false);
        $data = self::decode($answer->body);
        self::assertSame(['en', false], [$answer->locale, $answer->fallback]);
        self::assertSame('scene', $data['groups'][0]['label'], 'ref group replaced by the scene');
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
                    self::assertSame(['title' => 'Drei', 'number' => 3, 'release' => '2026-12-01', 'language' => 'de', 'status' => 'coming-soon'], json_decode((string) json_encode($error->extra['chapter']), true));
                }
            }
        }
        // draft mode (creators): no gates, the newest version
        $this->doc('t', '_', 'chapter2.json', ['title' => ['de' => 'Zwei neu'], 'number' => 2, 'groups' => []], false);
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

    /** Older data: a toon entry without `cover` keeps each language's cover from its toon settings */
    public function testCoverFromOlderToonSettings(): void
    {
        $this->seedToon();
        $this->doc('t', '_', 'index.json', self::ENTRY); // an entry without texts: the legacy toon settings are read
        $this->doc('t', 'en', 'toon.json', ['id' => 't', 'title' => 'T en', 'status' => 'published', 'cover' => ['square' => 'images/en.webp']]);
        $data = self::decode($this->library()->toon('t', 'en', false)->body);
        self::assertSame(['square' => 'images/en.webp'], $data['toon']['cover']);
    }

    /** Covers belong to the season (owner 2026-10-02): a cover left in the toon entry is not merged into the toon */
    public function testToonEntryCoverIsNotMerged(): void
    {
        $this->seedToon(['cover' => ['square' => 'images/s1-cover-square.webp', 'lettering' => true]]);
        $data = self::decode($this->library()->toon('t', 'de', false)->body);
        self::assertArrayNotHasKey('cover', $data['toon']);
    }

    /** The language-neutral settings (index entry) are merged into every locale's `toon`; `entry` stays the plain index entry */
    public function testToonBundleMergesTheNeutralSettings(): void
    {
        $map = ['layers' => ['terrain' => 'images/map/m.svg'], 'width' => 2048, 'height' => 1046, 'center' => ['x' => 1320, 'y' => 440]];
        $this->seedToon(['authors' => ['A'], 'chapters' => 5, 'thumbnail' => 'th.webp', 'map' => $map, 'tile' => ['image' => 'tile.webp', 'color' => '#0066cc']]);
        // a stale per-language document (legacy) is not read once the entry has texts
        $this->doc('t', 'en', 'toon.json', ['id' => 't', 'title' => 'T old', 'status' => 'published', 'map' => ['width' => 0]]);
        foreach (['de', 'en'] as $l) {
            $data = self::decode($this->library()->toon('t', $l, false)->body);
            self::assertSame("T $l", $data['toon']['title']);
            self::assertSame($map, $data['toon']['map'], "$l: the map of the index entry");
            self::assertSame(['A'], $data['toon']['authors']);
            self::assertSame(5, $data['toon']['chapters']);
            self::assertSame('th.webp', $data['toon']['thumbnail']);
            self::assertSame(['image' => 'tile.webp', 'color' => '#0066cc'], $data['toon']['tile']);
            self::assertSame(self::ENTRY + self::ENTRY_NAME, $data['entry'], 'entry without the settings, with the name');
        }
    }

    /**
     * The toon settings per language in the toon entry (agents/NEUTRAL.md step 4): the reader's `toon` is resolved from its
     * locale maps (the per-language documents are legacy), a language without a title falls back, `entry` stays plain
     */
    public function testToonSettingsFromTheEntry(): void
    {
        $this->seedToon(['title' => ['de' => 'T de', 'en' => 'T en', 'ko' => ' '], 'description' => ['de' => 'Geschichten', 'en' => ''],
            'release' => ['de' => '2025-10-04', 'en' => '2025-12-01'], 'authors' => ['A']]);
        $this->doc('t', 'en', 'toon.json', ['id' => 't', 'title' => 'T legacy', 'status' => 'published']);
        $en = $this->library()->toon('t', 'en', false);
        $data = self::decode($en->body);
        self::assertSame(['id' => 't', 'title' => 'T en', 'language' => 'en', 'release' => '2025-12-01', 'status' => 'published', 'description' => 'Geschichten', 'authors' => ['A']],
            $data['toon'], 'the per-language settings from the entry (a blank text → the source\'s) + the neutral settings');
        self::assertSame(self::ENTRY + self::ENTRY_NAME, $data['entry'], 'the entry without its texts and neutral settings, with the name');
        self::assertSame(['en', true], [$en->locale, $en->fallback], 'chapter 3 has no en title');
        $ko = $this->library()->toon('t', 'ko', false);
        self::assertSame(['de', 'T de', 'de'], [$ko->locale, self::decode($ko->body)['toon']['title'], self::decode($ko->body)['toon']['language']]);
        self::assertTrue($ko->fallback);
        $legacy = self::decode($this->library()->legacyOnly()->toon('t', 'en', false)->body);
        self::assertSame('T legacy', $legacy['toon']['title'], 'legacy answers read the per-language documents');
    }

    /**
     * The toon's one name (owner decision 4, 2026-10-06): the entry's `title`. The answered entries carry it per language
     * (resolved: a language without one has the source's) + the legacy `i18n.<l>.name` computed from it for older viewer
     * builds; an entry without texts (production before its conversion) keeps its stored `i18n`
     */
    public function testTheToonNameIsTheTitle(): void
    {
        $this->seedToon(['title' => ['de' => 'Katzenpapst', 'en' => ' '], 'i18n' => ['de' => ['name' => 'Stale', 'description' => '']]]);
        $name = ['title' => ['de' => 'Katzenpapst', 'en' => 'Katzenpapst'], 'i18n' => ['de' => ['name' => 'Katzenpapst'], 'en' => ['name' => 'Katzenpapst']]];
        $list = self::decode($this->library()->toons(false, 'en')->body);
        self::assertSame($name, ['title' => $list[0]['title'], 'i18n' => $list[0]['i18n']], 'the list: the title per language, a blank one → the source one; i18n computed (a stored one ignored)');
        $bundle = self::decode($this->library()->toon('t', 'en', false)->body);
        self::assertSame($name, ['title' => $bundle['entry']['title'], 'i18n' => $bundle['entry']['i18n']], 'the entry in the toon bundle likewise');
        self::assertSame('Katzenpapst', $bundle['toon']['title']);
        // legacy answers (content:neutral-check) see the entry as before the conversion: the name as i18n – the same answer
        $legacy = self::decode($this->library()->legacyOnly(false)->toons(true)->body)[0];
        self::assertSame($name, ['title' => $legacy['title'], 'i18n' => $legacy['i18n']]);
        // an entry without texts: its stored i18n names (also the title in the answer)
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'i18n' => ['de' => ['name' => 'Alt', 'description' => 'x'], 'en' => ['name' => 'Old']]]);
        foreach (['de', 'en'] as $l) $this->doc('t', $l, 'toon.json', ['id' => 't', 'title' => "T $l", 'status' => 'published']);
        $list = self::decode($this->library()->toons(false)->body);
        self::assertSame(['de' => 'Alt', 'en' => 'Old'], $list[0]['title']);
        self::assertSame(['de' => ['name' => 'Alt', 'description' => 'x'], 'en' => ['name' => 'Old']], $list[0]['i18n'], 'stored, untouched');
        // no `languages`: the title's keys are the languages (then the old i18n keys)
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'title' => ['en' => 'Cat Pope', 'de' => 'Katzenpapst']]);
        $en = $this->library()->toon('t', null, false);
        self::assertSame(['en', 'Cat Pope'], [$en->locale, self::decode($en->body)['toon']['title']], 'the source is the first title key');
    }

    /** A language's status closes the toon page in that language (404) and leaves the toon out of that language's list */
    public function testToonLanguageStatusGates(): void
    {
        $this->seedToon(['languageStatus' => ['de' => 'published', 'en' => 'draft']]);
        self::assertSame('T de', self::decode($this->library()->toon('t', 'de', false)->body)['toon']['title']);
        try {
            $this->library()->toon('t', 'en', false);
            self::fail('en is a draft: 404 expected');
        } catch (ApiError $error) {
            self::assertSame(404, $error->status);
        }
        self::assertSame('draft', self::decode($this->library()->toon('t', 'en', true)->body)['toon']['status'], 'creators see it');
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(false)->body), 'id'), 'no locale: as before');
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(false, 'de')->body), 'id'));
        self::assertSame([], self::decode($this->library()->toons(false, 'en')->body), 'closed in en: not listed in en');
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(true, 'en')->body), 'id'), 'draft mode lists it');
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(false, 'fr')->body), 'id'), 'fr is served in de: open');
        // a status only in the source is every language's; none at all = published
        $this->seedToon(['languageStatus' => ['de' => 'coming-soon']]);
        self::assertSame([], self::decode($this->library()->toons(false, 'en')->body));
        $entry = self::ENTRY + self::ENTRY_TEXTS;
        unset($entry['languageStatus']);
        $this->doc('t', '_', 'index.json', $entry);
        self::assertArrayNotHasKey('status', self::decode($this->library()->toon('t', 'en', false)->body)['toon']);
        // legacy toon settings gate the same way
        $this->doc('t', '_', 'index.json', self::ENTRY);
        $this->doc('t', 'de', 'toon.json', ['id' => 't', 'title' => 'T de', 'status' => 'published']);
        $this->doc('t', 'en', 'toon.json', ['id' => 't', 'title' => 'T en', 'status' => 'draft']);
        self::assertSame([], self::decode($this->library()->toons(false, 'en')->body));
        self::assertSame(['t'], array_column(self::decode($this->library()->toons(false, 'de')->body), 'id'));
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
        $this->doc('t', '_', 'chapter2.json', ['title' => ['de' => 'Zwei de', 'en' => 'Zwei en'], 'number' => 2, 'groups' => [], 'featuring' => ['star', 'crows']]);
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
        $this->seedSeason(['download' => $download, 'supporters' => $supporters, 'trailer' => $trailer]);
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

        $this->doc('t', '_', 'chapter1/scene-1.json', ['label' => 'scene v2'], true);
        $after = $this->library()->chapter('t', 1, 'de', false);
        self::assertNotSame($first->etag, $after->etag, 'a publish changes the key');
        self::assertSame('scene v2', self::decode($after->body)['groups'][0]['label']);
    }

    /** The screenplay (creator material) is never in a library answer and saving it changes no reader cache key */
    public function testScriptsAreNeverInTheLibrary(): void
    {
        $this->seedToon();
        $answers = fn(bool $draft) => [$this->library()->toon('t', 'de', $draft), $this->library()->chapter('t', 1, 'de', $draft), $this->library()->season('t', 1, 'de', $draft)];
        $before = array_map(fn($a) => [$a->etag, $a->body], $answers(false));
        $this->doc('t', 'de', 'script.json', ['text' => "    # 1. Kapitel\nGEHEIMES DREHBUCH\n", 'meta' => ['title' => 'Geheim']]);
        $this->doc('t', 'en', 'script.json', ['text' => 'SECRET SCREENPLAY'], false);
        self::assertSame($before, array_map(fn($a) => [$a->etag, $a->body], $answers(false)), 'same answers and cache keys after a script publish');
        foreach ([...$answers(false), ...$answers(true)] as $answer) {
            self::assertStringNotContainsString('GEHEIM', $answer->body);
            self::assertStringNotContainsString('SECRET', $answer->body);
        }
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
        $this->seedToon(['map' => ['center' => ['x' => 100, 'y' => 50]]]);
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
        self::assertSame(['center' => ['x' => 100, 'y' => 50], 'zoom' => 1], $view, 'no colour without a tile colour or a star');
        // the sea is the toon's colour: its star's registry colour, a tile colour wins (owner 2026-10-01)
        $this->doc(Library::PLATFORM, '_', 'characters.json', ['characters' => [['id' => 'hero', 'own_toon' => 't', 'color' => 'blue']]]);
        self::assertSame('blue', self::decode($this->library()->map('de', 't', false)->body)['view']['color']);
        $this->doc('t', '_', 'index.json', ['id' => 't', 'uri' => 't', 'status' => 'published', 'tile' => ['image' => 'a.webp', 'color' => '#dd5ba2']]);
        self::assertSame('#dd5ba2', self::decode($this->library()->map('de', 't', false)->body)['view']['color']);
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

    /**
     * The world map as documents (agents/MAP-DOCUMENTS.md): place:<id> / route:<id> on their own – a newer draft of one
     * place only in draft mode, a removed place (tombstone) is gone once published, a new place joins at the end
     * (creation order), and a place publish gives the answer a new cache key
     */
    public function testWorldMapFromDocuments(): void
    {
        $this->seedWorldMap();
        $store = $this->get(EntityStore::class);
        $ids = fn(bool $draft) => array_column(self::decode($this->library()->map('de', null, $draft)->body)['locations'], 'id');
        $before = $this->library()->map('de', null, false)->etag;

        // a draft of one place: readers still get the published one
        $store->insert(EntityKey::parse('place:home'), (string) json_encode(['name' => ['de' => 'Neu'], 'x' => 91, 'y' => 40, 'toons' => ['t']]), null, 'draft', null);
        self::assertSame('Zuhause', self::decode($this->library()->map('de', null, false)->body)['locations'][0]['name']);
        self::assertSame('Neu', self::decode($this->library()->map('de', null, true)->body)['locations'][0]['name']);
        self::assertSame($before, $this->library()->map('de', null, false)->etag, 'a draft changes nothing for readers');

        // a new place: at the end; a removed one: a published tombstone
        $new = $store->insert(EntityKey::parse('place:well'), (string) json_encode(['name' => ['de' => 'Brunnen'], 'x' => 95, 'y' => 45, 'toons' => ['t']]), null, 'draft', null);
        $store->setPublished(['id' => $new, 'entity_key' => 'place:well']);
        $gone = $store->insert(EntityKey::parse('place:tower'), (string) json_encode(['name' => ['de' => 'Turm'], 'x' => 120, 'y' => 60, 'toons' => ['t'], 'removed' => true]), null, 'draft', null);
        self::assertSame(['home', 'tower', 'well'], $ids(false), 'the tombstone is a draft: tower is still there');
        $store->setPublished(['id' => $gone, 'entity_key' => 'place:tower']);
        self::assertSame(['home', 'well'], $ids(false));
        self::assertNotSame($before, $this->library()->map('de', null, false)->etag, 'publishing a place gives a new cache key');

        // the old bundle lists follow the documents too
        self::assertSame(['home', 'well'], array_column(self::decode($this->library()->toon('t', 'en', false)->body)['locations'], 'id'));
    }

    public function testPlaceAndRouteKeys(): void
    {
        self::assertSame('cathedral', EntityKey::parse('place:cathedral')->id);
        self::assertSame('place:cathedral', (string) EntityKey::parse('place:cathedral'));
        self::assertSame('route:season1', EntityKey::parse('route:season1')->entity());
        self::assertNull(EntityKey::parse('place:cathedral')->toon, 'platform-wide');
        foreach (['place', 'place:', 'place:A', 'place:a:b', 'route:-x', 'place:scutoons:de'] as $bad) self::assertFalse(EntityKey::isValid($bad), $bad);
    }

    // ── Language-neutral chapters, scenes, seasons (agents/NEUTRAL.md) ─────────────────────────────────────

    /** Neutral documents for seedLegacy()'s season 1, chapter 1 and its scene (texts per locale; en: chapter 1 coming soon) */
    private function seedNeutral(bool $scene = true): void
    {
        $this->doc('t', '_', 'season1.json', ['title' => ['de' => 'NS de', 'en' => 'NS en'], 'number' => 1, 'sequence' => [
            ['number' => 1, 'status' => ['de' => 'published', 'en' => 'coming-soon']],
            ['number' => 2, 'status' => ['de' => 'draft', 'en' => 'draft']],
            ['number' => 3, 'status' => ['de' => 'coming-soon']],
        ]]);
        $this->doc('t', '_', 'chapter1.json', ['title' => ['de' => 'N Eins', 'en' => 'N One'], 'number' => 1, 'release' => ['de' => '2025-10-04', 'en' => '2025-11-01'],
            'description' => ['de' => 'Beschreibung'], 'thumbnail' => 'x.webp', 'featuring' => ['catpope'], 'audio' => ['volume' => 1.0],
            'groups' => [['ref' => ['src' => 'chapter1/scene-1.json']], ['label' => 'inline', 'slices' => [['image' => 'a.webp', 'width' => 1, 'height' => 1,
                'dialog' => [['rect' => ['x' => 0, 'y' => 0, 'width' => 1], 'text' => ['de' => 'Hallo', 'en' => 'Hello']]]]]]]]);
        if ($scene) {
            $this->doc('t', '_', 'chapter1/scene-1.json', ['label' => 'N scene', 'events' => [['type' => 'showMessage', 'params' => ['text' => ['de' => 'Na?'], 'character' => 'plf']]]]);
        }
    }

    public function testNeutralChapterWinsOverThePerLocaleDocuments(): void
    {
        $this->seedLegacy();
        $this->seedNeutral();
        $answer = $this->library()->chapter('t', 1, 'de', false);
        $data = self::decode($answer->body);
        self::assertSame(['de', false, []], [$answer->locale, $answer->fallback, $answer->missing]);
        self::assertSame(['N Eins', 'de', 'published', '2025-10-04', 'Beschreibung'], [$data['title'], $data['language'], $data['status'], $data['release'], $data['description']]);
        self::assertSame('N scene', $data['groups'][0]['label'], 'the neutral scene is inlined');
        self::assertSame('Na?', $data['groups'][0]['events'][0]['params']['text']);
        self::assertSame('Hallo', $data['groups'][1]['slices'][0]['dialog'][0]['text']);
        self::assertStringContainsString('"volume":1.0', $answer->body);

        // en: the neutral season says coming soon (the per-locale en season said published)
        try {
            $this->library()->chapter('t', 1, 'en', false);
            self::fail('coming_soon expected');
        } catch (ApiError $error) {
            self::assertSame('coming_soon', $error->errorCode);
            self::assertSame('N One', $error->extra['chapter']->title);
        }
        // creators (draft mode): en is served, its scene falls back to de (no en text in it)
        $en = $this->library()->chapter('t', 1, 'en', true);
        $data = self::decode($en->body);
        self::assertSame(['en', true], [$en->locale, $en->fallback], 'the scene has no en text → fallback');
        self::assertSame(['N One', 'en', 'coming-soon', 'Beschreibung', 'Hello', 'Na?'],
            [$data['title'], $data['language'], $data['status'], $data['description'], $data['groups'][1]['slices'][0]['dialog'][0]['text'], $data['groups'][0]['events'][0]['params']['text']]);

        // the legacy answer is still there for the equivalence check (content:neutral-check)
        $legacy = self::decode($this->library()->legacyOnly()->chapter('t', 1, 'en', false)->body);
        self::assertSame(['Eins en', 'scene en'], [$legacy['title'], $legacy['groups'][0]['label']]);
        self::assertNotSame($this->library()->chapter('t', 1, 'de', false)->etag, $this->library()->legacyOnly()->chapter('t', 1, 'de', false)->etag, 'another cache key');
    }

    public function testNeutralFallbackWhenTheLocaleHasNoTitle(): void
    {
        $this->seedLegacy();
        $this->doc('t', '_', 'chapter1.json', ['title' => ['de' => 'Nur de', 'en' => ' '], 'number' => 1, 'release' => ['de' => '2025-10-04'], 'groups' => []]);
        $answer = $this->library()->chapter('t', 1, 'en', false);
        $data = self::decode($answer->body);
        self::assertSame(['de', true], [$answer->locale, $answer->fallback]);
        self::assertSame(['Nur de', 'de', 'published'], [$data['title'], $data['language'], $data['status']], 'served in de: the de season status');
        $fr = $this->library()->chapter('t', 1, 'fr', false);
        self::assertSame(['de', true], [$fr->locale, $fr->fallback]);
    }

    public function testNeutralChapterWithLegacySceneAndBack(): void
    {
        $this->seedLegacy();
        $this->seedNeutral(false);
        $answer = $this->library()->chapter('t', 1, 'de', false);
        $data = self::decode($answer->body);
        self::assertSame(['N Eins', 'scene de', false], [$data['title'], $data['groups'][0]['label'], $answer->fallback], 'no neutral scene → the per-locale one');
        // and a legacy chapter with a neutral scene
        $this->setUp();
        $this->seedLegacy();
        $this->doc('t', '_', 'chapter1/scene-1.json', ['label' => 'N scene']);
        $data = self::decode($this->library()->chapter('t', 1, 'en', false)->body);
        self::assertSame(['Eins en', 'N scene'], [$data['title'], $data['groups'][0]['label']]);
    }

    public function testNeutralSeasonInTheToonBundleCastAndSummaries(): void
    {
        $this->seedLegacy();
        $this->seedNeutral();
        $de = self::decode($this->library()->toon('t', 'de', false)->body);
        self::assertCount(1, $de['seasons'], 'a season stored both ways is one season');
        $season = $de['seasons'][0];
        self::assertSame(['NS de', 'de'], [$season['season']['title'], $season['season']['language']]);
        self::assertSame([1, 3], array_column($season['chapters'], 'number'));
        self::assertSame(['title' => 'N Eins', 'number' => 1, 'release' => '2025-10-04', 'description' => 'Beschreibung', 'thumbnail' => 'x.webp', 'featuring' => ['catpope'],
            'language' => 'de', 'status' => 'published'], $season['chapters'][0], 'summary of the neutral chapter: no groups / audio');
        self::assertSame('Drei', $season['chapters'][1]['title'], 'chapter 3 stays per locale');
        $en = self::decode($this->library()->toon('t', 'en', false)->body)['seasons'][0];
        self::assertSame(['NS en', 'coming-soon', 'N One', '2025-11-01'], [$en['season']['title'], $en['chapters'][0]['status'], $en['chapters'][0]['title'], $en['chapters'][0]['release']]);
        self::assertSame('coming-soon', $en['season']['sequence'][2]['status'], "a status missing in en → the source's");
        self::assertSame('published', self::decode($this->library()->season('t', 1, 'de', false)->body)['chapters'][0]['status']);
        // cast?chapter= reads the neutral chapter (its featuring + the neutral scene's characters)
        $this->doc('t', '_', 'cast.json', ['seasons' => ['1' => []]]);
        $this->doc('scutoons', '_', 'characters.json', ['characters' => [['id' => 'catpope'], ['id' => 'other']]]);
        $cast = self::decode($this->library()->cast('t', 1, false)->body);
        self::assertSame(['catpope'], array_column($cast['registry']['characters'], 'id'));
    }

    public function testImportSourcesAndExportRoundTrip(): void
    {
        $root = $this->dir . '/static';
        mkdir("$root/t/de/chapter1", 0777, true);
        mkdir("$root/t/_/chapter2", 0777, true);
        file_put_contents("$root/t/_/chapter2.json", '{"title":{"de":"Zwei"},"groups":[]}');
        file_put_contents("$root/t/_/chapter2/kiosk.json", '{"label":"kiosk"}');
        file_put_contents("$root/t/_/season2.json", '{"title":{"de":"S2"},"sequence":[]}');
        file_put_contents("$root/toons.json", json_encode([['id' => 'b', 'status' => 'published'], ['id' => 'a', 'status' => 'draft'], ['id' => 'Bad!']]));
        file_put_contents("$root/events.json", '[{"title":"E"}]');
        foreach (['toon', 'season1', 'chapter1', 'locations', 'routes', 'collectables', 'characters', 'credits'] as $name) {
            file_put_contents("$root/t/de/$name.json", json_encode(['name' => $name]));
        }
        file_put_contents("$root/t/de/chapter1/scene-1.json", '{"params":{}}');
        $keys = array_column(ContentFiles::sources($root), 0);
        self::assertSame(['toon:b', 'toon:a', 'events', 'chapter:t:2', 'scene:t:2:kiosk', 'season:t:2', 'chapter:t:1:de', 'scene:t:1:scene-1:de',
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
