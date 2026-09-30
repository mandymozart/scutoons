<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Console\ImportContentCommand;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
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

    /** Stores a document version (published unless told otherwise); returns its id */
    private function doc(string $toon, string $locale, string $path, mixed $data, bool $publish = true): int
    {
        $content = $this->get(ContentService::class);
        $json = json_encode($data, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
        $id = $content->insert($toon, $locale, $path, (string) $json, null, 'draft', null);
        if ($publish) $content->setPublished(['id' => $id, 'toon' => $toon, 'locale' => $locale, 'path' => $path]);
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
        self::assertSame(['chapter1/gone.json'], $answer->missing);
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
        $keys = array_map(fn(array $s) => "$s[0]/$s[1]/$s[2]", ImportContentCommand::sources($root));
        self::assertSame(['b/_/index.json', 'a/_/index.json', 'scutoons/_/events.json', 't/de/chapter1.json', 't/de/chapter1/scene-1.json',
            't/de/locations.json', 't/de/routes.json', 't/de/season1.json', 't/de/toon.json'], $keys,
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
        $keysAgain = array_map(fn(array $s) => "$s[0]/$s[1]/$s[2]", ImportContentCommand::sources($out));
        self::assertSame($keys, $keysAgain, 'the export reads back as the same documents');
    }
}
