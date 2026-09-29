<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\ContentService;
use Scutoons\Tests\TestCase;

final class ContentServiceTest extends TestCase
{
    public function testSavePublishRestore(): void
    {
        $content = $this->get(ContentService::class);
        $user = $this->user();
        $body = (object) ['content' => (object) ['title' => 'Eins', 'params' => new \stdClass(), 'volume' => 0.5], 'base' => null, 'message' => "  first\n draft "];
        $v1 = $content->save($user, 'smoke', 'de', 'chapter1.json', $body);
        self::assertSame(['draft', 'first draft', false], [$v1['version']['status'], $v1['version']['message'], $v1['unchanged']]);
        $id1 = $v1['version']['id'];
        self::assertTrue($content->save($user, 'smoke', 'de', 'chapter1.json', (object) ['content' => $body->content])['unchanged']);
        self::assertSame('{"title":"Eins","params":{},"volume":0.5}', $content->latest('smoke', 'de', 'chapter1.json')['json'] ?? null);

        try {
            $content->save($user, 'smoke', 'de', 'chapter1.json', (object) ['content' => ['x' => 1], 'base' => $id1 - 1]);
            self::fail('conflict expected');
        } catch (ApiError $error) {
            self::assertSame(['conflict', $id1], [$error->errorCode, $error->extra['latest']['id']]);
        }
        self::assertNull($content->published('smoke', 'de', 'chapter1.json'));
        self::assertSame('published', $content->publish((object) ['toon' => 'smoke', 'locale' => 'de', 'path' => 'chapter1.json'])['version']['status']);
        $v2 = $content->save($user, 'smoke', 'de', 'chapter1.json', (object) ['content' => ['title' => 'Zwei'], 'base' => $id1]);
        $restored = $content->restore($user, (object) ['id' => $id1]);
        self::assertSame("restore #$id1", $restored['version']['message']);
        $live = $content->published('smoke', 'de', 'chapter1.json');
        self::assertNotNull($live);
        self::assertSame($id1, (int) $live['id'], 'restore does not publish');
        self::assertSame(3, $content->index('smoke')['files'][0]['versions']);
        self::assertSame([$restored['version']['id'], $v2['version']['id'], $id1], array_column($content->versions(['toon' => 'smoke', 'locale' => 'de', 'path' => 'chapter1.json'])['versions'], 'id'));
        self::assertSame(1, $content->unpublish((object) ['toon' => 'smoke', 'locale' => 'de', 'path' => 'chapter1.json'])['unpublished']);
    }

    public function testKeysAndBodiesAreValidated(): void
    {
        foreach ([['Bad', 'de', 'a.json', 'invalid_toon'], ['t', 'deu', 'a.json', 'invalid_locale'], ['t', '__', 'a.json', 'invalid_locale'], ['t', 'de', 'a/b/c.json', 'invalid_path'], ['t', 'de', 'A.json', 'invalid_path']] as [$toon, $locale, $path, $code]) {
            try {
                ContentService::checkKey($toon, $locale, $path);
                self::fail("$code expected");
            } catch (ApiError $error) {
                self::assertSame($code, $error->errorCode);
            }
        }
        ContentService::checkKey('t', 'de', 'chapter2/scene-14.json');
        ContentService::checkKey('t', ContentService::NEUTRAL_LOCALE, 'index.json');
        self::assertSame(['t', '_', 'index.json'], ContentService::parseFile('t/_/index.json'));
        self::assertSame(['t', 'de', 'chapter2/scene-14.json'], ContentService::parseFile('t/de/chapter2/scene-14.json'));
        $content = $this->get(ContentService::class);
        try {
            $content->encode('a string');
            self::fail('invalid_content expected');
        } catch (ApiError $error) {
            self::assertSame(422, $error->status);
        }
    }
}
