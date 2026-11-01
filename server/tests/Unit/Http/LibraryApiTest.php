<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Assets\AssetRegistry;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Infrastructure\AssetStorage\StoredFile;
use Scutoons\Console\ContentFiles;
use Scutoons\Domain\Content\CollectionDocs;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/** /api/library/* and /api/events over HTTP: headers, ETag / 304, CORS, draft mode, errors */
final class LibraryApiTest extends TestCase
{
    /** @param array<string, string> $headers */
    private function request(string $uri, array $headers = []): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest('GET', "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    private function publish(string $toon, string $locale, string $path, mixed $data, bool $publish = true): void
    {
        $key = match (true) {
            $path === 'index.json' => "toon:$toon",
            $path === 'events.json' => 'events',
            default => (string) ContentFiles::keyOf($toon, $locale, $path),
        };
        $store = $this->get(EntityStore::class);
        $collection = CollectionDocs::ofAnchor($key);
        // the registry: the anchor + one document per character, like an import
        $parts = $collection !== null ? CollectionDocs::split($collection, json_decode((string) json_encode($data))) : [$key => $data];
        foreach ($parts as $part => $doc) {
            $id = $store->insert(EntityKey::parse((string) $part), (string) json_encode($doc), null, 'draft', null);
            if ($publish) $store->setPublished(['id' => $id, 'entity_key' => (string) $part]);
        }
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->publish('t', '_', 'index.json', ['id' => 't', 'status' => 'published', 'languages' => ['de', 'en']]);
        $this->publish('t', 'de', 'toon.json', ['id' => 't', 'status' => 'published']);
        $this->publish('t', 'de', 'season1.json', ['sequence' => [['number' => 1, 'status' => 'published'], ['number' => 2, 'status' => 'draft']]]);
        $this->publish('t', 'de', 'chapter1.json', ['title' => 'Eins', 'groups' => []]);
        $this->publish('t', 'de', 'chapter2.json', ['title' => 'Zwei', 'groups' => []]);
        $this->publish('scutoons', '_', 'events.json', [['title' => 'E']]);
    }

    public function testPublicAnswerHeadersAndConditionalGet(): void
    {
        $r = $this->request('/api/library/toons/t/chapters/1?locale=en', ['Origin' => 'https://elsewhere.test']);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['title' => 'Eins', 'groups' => []], self::json($r));
        self::assertSame('application/json; charset=utf-8', $r->getHeaderLine('Content-Type'));
        self::assertSame('public, max-age=60', $r->getHeaderLine('Cache-Control'));
        self::assertSame(['de', '1'], [$r->getHeaderLine('Content-Language'), $r->getHeaderLine('X-Content-Fallback')]);
        self::assertSame('*', $r->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertStringContainsString('ETag', $r->getHeaderLine('Access-Control-Expose-Headers'));
        $etag = $r->getHeaderLine('ETag');
        self::assertMatchesRegularExpression('/^"l[0-9a-f]{24}"$/', $etag);

        $again = $this->request('/api/library/toons/t/chapters/1?locale=en', ['If-None-Match' => $etag]);
        self::assertSame([304, ''], [$again->getStatusCode(), (string) $again->getBody()]);
        self::assertSame(200, $this->request('/api/library/toons/t/chapters/1', ['If-None-Match' => '"other"'])->getStatusCode());
        self::assertSame('http://viewer.test', $this->request('/api/library/toons', ['Origin' => 'http://viewer.test'])->getHeaderLine('Access-Control-Allow-Origin'));
        self::assertSame([['title' => 'E']], self::json($this->request('/api/events/')), 'trailing slash like every endpoint');
    }

    public function testErrorsAndGates(): void
    {
        $r = $this->request('/api/library/toons/t/chapters/2');
        self::assertSame([403, 'not_public'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertSame('no-store', $r->getHeaderLine('Cache-Control'));
        self::assertSame(400, $this->request('/api/library/toons/t/chapters/1?locale=deu')->getStatusCode());
        self::assertSame('invalid_locale', self::json($this->request('/api/library/toons/t?locale=DE'))['error']);
        self::assertSame('invalid_chapter', self::json($this->request('/api/library/toons/t/cast?chapter=x'))['error']);
        self::assertSame(404, $this->request('/api/library/toons/t/chapters/one')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/nope')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/t/seasons/7')->getStatusCode());
        self::assertSame(404, $this->request('/api/library/toons/t/credits')->getStatusCode());
    }

    public function testDraftModeNeedsTheCreatorRole(): void
    {
        $this->publish('t', 'de', 'chapter2.json', ['title' => 'Zwei (neu)', 'groups' => []], false);
        self::assertSame([401, 'unauthorized'], [($r = $this->request('/api/library/toons/t/chapters/2?draft=1'))->getStatusCode(), self::json($r)['error']]);
        $user = $this->user();
        $token = $this->get(Sessions::class)->create((string) $user['id']);
        self::assertSame(403, $this->request('/api/library/toons/t/chapters/2?draft=1', ['Authorization' => "Bearer $token"])->getStatusCode());
        $this->get(Roles::class)->grant((string) $user['id'], Roles::CREATOR);
        $r = $this->request('/api/library/toons/t/chapters/2?draft=1', ['Authorization' => "Bearer $token"]);
        self::assertSame([200, 'Zwei (neu)'], [$r->getStatusCode(), self::json($r)['title']]);
        self::assertSame('private, no-store', $r->getHeaderLine('Cache-Control'));
    }

    public function testChapterFilesForSaveToDevice(): void
    {
        $this->publish('t', 'de', 'chapter1.json', [
            'title' => 'Eins',
            'thumbnail' => 's1/thumb.jpg',
            'audio' => [
                'music' => ['tracks' => [['id' => 'a', 'path' => 's1/a.128.mp3', 'variants' => ['low' => 's1/a.96.mp3', 'high' => 's1/a.320.mp3']]]],
                'sampleLibrary' => [['id' => 'b', 'path' => 's1/b.128.mp3']],
            ],
            'groups' => [[
                'slices' => [
                    ['image' => 's1/one.webp', 'imageVariants' => ['medium' => 's1/one@2x.webp', 'high' => 's1/one@3x.webp'], 'items' => [['item' => 'cup']]],
                    ['image' => 's1/two.webp', 'video' => ['src' => 's1/v.720p.mp4', 'variants' => ['low' => 's1/v.480p.mp4', 'medium' => 's1/v.720p.mp4']]],
                    ['image' => '../other/x.webp', 'download' => 's1/album.zip'],
                ],
                'events' => [['type' => 'joinScene', 'params' => ['character' => 'hero', 'persona' => 'masked']]],
            ]],
        ]);
        $this->publish('scutoons', '_', 'characters.json', ['characters' => [
            ['id' => 'hero', 'image' => 'characters/hero/hero.svg', 'personas' => [
                ['id' => 'regular', 'looks' => [['id' => 'default', 'images' => ['neutral' => ['avatar' => 'characters/hero/a.webp', 'portrait' => 'characters/hero/p.webp', 'full' => 'characters/hero/f.webp']], 'apose' => ['image' => 'characters/hero/apose.webp']]]],
                ['id' => 'masked', 'looks' => [['id' => 'default', 'images' => ['neutral' => ['portrait' => 'characters/hero/m.webp']]]]],
                ['id' => 'old', 'looks' => [['id' => 'default', 'images' => ['neutral' => ['portrait' => 'characters/hero/old.webp']]]]],
            ]],
            ['id' => 'extra', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'images' => ['neutral' => ['portrait' => 'characters/extra/p.webp']]]]]]],
        ]]);
        $this->publish('t', '_', 'cast.json', ['seasons' => ['1' => [['character' => 'hero'], ['character' => 'extra']]]]);
        $registry = $this->get(AssetRegistry::class);
        $asset = $registry->create('t/s1/one', 'image');
        $version = $registry->addVersion($asset, new StoredFile('t/s1/one.webp', 100), 'local');
        $registry->addRendition($asset, $version, '1x', 'low', new StoredFile('t/s1/one.webp', 100));
        $registry->addRendition($asset, $version, '3x', 'high', new StoredFile('t/s1/one@3x.webp', 900));

        $r = $this->request('/api/library/toons/t/chapters/1/files?locale=de');
        self::assertSame(200, $r->getStatusCode());
        self::assertSame('public, max-age=60', $r->getHeaderLine('Cache-Control'));
        $body = self::json($r);
        self::assertSame(['t', 1, 'de', true], [$body['toon'], $body['chapter'], $body['locale'], $body['gated']]);
        $files = [];
        foreach ($body['files'] as $f) $files[$f['path']] = [$f['kind'], $f['levels'] ?? 'all', $f['size']];
        $groups = [];
        foreach ($body['files'] as $f) if (isset($f['group'])) $groups[$f['group']][] = $f['path'];
        self::assertSame([['t/s1/a.320.mp3', 't/s1/a.96.mp3'], ['t/s1/one.webp', 't/s1/one@2x.webp', 't/s1/one@3x.webp'], ['t/s1/v.480p.mp4', 't/s1/v.720p.mp4']],
            array_values(array_map(function (array $g): array {
                sort($g);
                return $g;
            }, $groups)), 'one group per media with levels');
        self::assertSame([
            'characters/hero/a.webp' => ['image', 'all', null],
            'characters/hero/hero.svg' => ['image', 'all', null],
            'characters/hero/m.webp' => ['image', 'all', null],
            'characters/hero/p.webp' => ['image', 'all', null],
            'other/x.webp' => ['image', 'all', null],
            't/s1/a.320.mp3' => ['audio', ['high'], null],
            't/s1/a.96.mp3' => ['audio', ['low', 'medium'], null],
            't/s1/b.128.mp3' => ['audio', 'all', null],
            't/s1/one.webp' => ['image', ['low'], 100],
            't/s1/one@2x.webp' => ['image', ['medium'], null],
            't/s1/one@3x.webp' => ['image', ['high'], 900],
            't/s1/thumb.jpg' => ['image', 'all', null],
            't/s1/two.webp' => ['image', 'all', null],
            't/s1/v.480p.mp4' => ['video', ['low'], null],
            't/s1/v.720p.mp4' => ['video', ['medium', 'high'], null],
        ], $files, 'per level the file pickQualityVariant plays (no medium track: the low one, never the default); VN personas + regular, no full view / A-pose / other personas; no zip');

        // the gates of the chapter itself: a draft chapter and a coming-soon one have no files
        self::assertSame([403, 'not_public'], [($d = $this->request('/api/library/toons/t/chapters/2/files'))->getStatusCode(), self::json($d)['error']]);
        self::assertSame(404, $this->request('/api/library/toons/nope/chapters/1/files')->getStatusCode());
        $this->publish('t', 'de', 'chapter3.json', ['title' => 'Drei', 'isPreview' => true, 'groups' => []]);
        $this->publish('t', 'de', 'season1.json', ['sequence' => [['number' => 1, 'status' => 'published'], ['number' => 2, 'status' => 'coming-soon'], ['number' => 3, 'status' => 'published']]]);
        self::assertSame([403, 'coming_soon'], [($c = $this->request('/api/library/toons/t/chapters/2/files'))->getStatusCode(), self::json($c)['error']]);
        self::assertFalse(self::json($this->request('/api/library/toons/t/chapters/3/files'))['gated'], 'a free preview chapter');
    }
    /** #195: what a deck's scene needs – the cathedral set of the migration, a character's portraits, sizes from the registry */
    public function testSceneContentForDecks(): void
    {
        $this->publish('scutoons', '_', 'characters.json', ['characters' => [
            ['id' => 'catpope', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'default' => true, 'images' => [
                'neutral' => ['avatar' => 'characters/catpope/catpope_avatar-neutral.webp', 'portrait' => 'characters/catpope/catpope_portrait-neutral.webp'],
                'happy' => ['portrait' => 'characters/catpope/catpope_portrait-happy.webp'],
            ]]]]]],
        ]]);
        $registry = $this->get(AssetRegistry::class);
        $asset = $registry->create('catpope/map/sets/cathedral.low.v2', 'model');
        $version = $registry->addVersion($asset, new StoredFile('catpope/map/sets/cathedral.low.v2.glb', 516928), 'local');
        $registry->addRendition($asset, $version, '1x', 'low', new StoredFile('catpope/map/sets/cathedral.low.v2.glb', 516928));

        $r = $this->request('/api/library/scene-content?sets=cathedral,nowhere&characters=plf,ghost&items=incense_burner,Ghost', ['Authorization' => 'Bearer scd_not-checked-for-public-content']);
        self::assertSame([200, 'public, max-age=60', '*'], [$r->getStatusCode(), $r->getHeaderLine('Cache-Control'), $r->getHeaderLine('Access-Control-Allow-Origin')]);
        $body = self::json($r);
        self::assertSame([
            'low' => ['path' => 'catpope/map/sets/cathedral.low.v2.glb', 'size' => 516928],
            'medium' => ['path' => 'catpope/map/sets/cathedral.medium.v2.glb', 'size' => null],
            'high' => ['path' => 'catpope/map/sets/cathedral.high.v2.glb', 'size' => null],
        ], $body['sets']['cathedral']['model'], 'the set of the migrations 2026-10-30-cathedral-set + 2026-10-31-cathedral-set-v2');
        self::assertSame(['character' => 'catpope', 'persona' => 'regular', 'look' => 'default', 'portraits' => [
            'happy' => ['path' => 'characters/catpope/catpope_portrait-happy.webp', 'size' => null],
            'neutral' => ['path' => 'characters/catpope/catpope_portrait-neutral.webp', 'size' => null],
        ]], array_diff_key($body['characters']['plf'], ['rig' => 1]), 'an old VN id → catpope');
        // the rigs (#258): catpope's core rig of the migration 2026-11-01-rigs_core, the incense burner's (an item)
        self::assertSame(['character.catpope', ['cathedral_voice', 'drone']], [$body['characters']['plf']['rig']['id'], $body['characters']['plf']['rig']['instruments']]);
        self::assertSame(['incense'], $body['items']['incense_burner']['rig']['instruments']);
        self::assertSame(['cathedral_voice', 'drone', 'incense'], array_keys($body['instruments']));
        self::assertSame(['sets' => ['nowhere'], 'characters' => ['ghost'], 'items' => ['Ghost'], 'presets' => []], $body['missing']);
        $again = $this->request('/api/library/scene-content?sets=cathedral,nowhere&characters=plf,ghost&items=incense_burner,Ghost', ['If-None-Match' => $r->getHeaderLine('ETag')]);
        self::assertSame(304, $again->getStatusCode());

        self::assertSame('{"sets":{},"characters":{},"items":{},"instruments":{},"presets":{},"missing":{"sets":[],"characters":[],"items":[],"presets":[]}}', (string) $this->request('/api/library/scene-content')->getBody(), 'maps stay objects');
        self::assertSame(400, $this->request('/api/library/scene-content?items=../x')->getStatusCode());
        self::assertSame([400, 'invalid_ids'], [($bad = $this->request('/api/library/scene-content?sets=../x'))->getStatusCode(), self::json($bad)['error']]);
        self::assertSame(400, $this->request('/api/library/scene-content?characters=' . implode(',', array_map(fn($i) => "c$i", range(1, 33))))->getStatusCode());
    }
}
