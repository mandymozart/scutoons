<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Live\SceneUses;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Tests\FakeCore;
use Scutoons\Tests\TestCase;

/**
 * #504: a use of a scene is checked by the Rust core (the build service's `/validate`, kind `uses`), never by PHP – the
 * requests and the core's answers are the shared vectors server/tests/fixtures/scenes/uses.json (deck-space checks the
 * same lines): the API sends the use, the scene's published document and the toon's book, and refuses with the core's words.
 */
final class SceneUsesTest extends TestCase
{
    private FakeCore $core;

    protected function settings(): array
    {
        return FakeCore::settings() + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->core = new FakeCore();
        $this->container->set(HttpClient::class, $this->core);
    }

    public function testTheCoreDecidesWithItsWords(): void
    {
        $this->store('toon:catpope', ['id' => 'catpope', 'sourceLanguage' => 'en', 'title' => ['en' => 'Cat Pope']]);
        $this->store('cast:catpope', ['seasons' => ['1' => [['character' => 'catpope', 'personas' => ['regular']]]]]);
        $this->store('character:catpope', ['name' => ['en' => 'Cat Pope'], 'color' => 'blue', 'personas' => [['id' => 'regular', 'looks' => []]]]);
        $this->store('place:dom', ['name' => ['en' => 'Dom'], 'toons' => ['catpope'], 'x' => 1, 'y' => 2]);
        $cases = (array) json_decode((string) file_get_contents(self::root() . '/tests/fixtures/scenes/uses.json'), true)['cases'];
        $this->store('scene:stage', $cases[0]['request']['scenes'][0]);
        $uses = $this->get(SceneUses::class);
        foreach ($cases as $case) {
            $this->core->requests = [];
            $this->core->answer = fn(array $request): array => $case['problems'];
            $use = json_decode((string) json_encode($case['request']['document']));
            $scenes = $case['request']['scenes'];
            if ($scenes === []) $this->get(EntityStore::class)->unpublish('scene:stage');
            try {
                $ok = $uses->check('catpope', $use);
                self::assertSame([], $case['problems'], "{$case['name']}: passed");
                self::assertSame(['scene' => 'scene:stage', 'version' => 1], $ok);
            } catch (ApiError $e) {
                self::assertNotSame([], $case['problems'], "{$case['name']}: refused");
                self::assertSame([422, 'invalid_use'], [$e->status, $e->errorCode], $case['name']);
                self::assertSame($case['problems'], $e->extra['problems'] ?? null, "{$case['name']}: the core's words, unchanged");
            }
            $sent = $this->core->requests[0];
            self::assertSame('POST ' . FakeCore::URL . '/validate', $sent['url']);
            self::assertEquals(['kind' => 'uses', 'id' => 'stage', 'document' => $case['request']['document'], 'scenes' => $scenes], array_diff_key($sent['request'], ['book' => 1]), $case['name']);
            // the toon's book as it stands: scene:stage is in it while published (a template), out once unpublished above
            $book = array_merge(['location:dom', 'character:catpope'], $scenes !== [] ? ['scene:stage'] : []);
            self::assertSame([], array_diff($book, $sent['request']['book']), "{$case['name']}: the toon's book");
            if ($scenes === []) self::assertNotContains('scene:stage', $sent['request']['book'], 'an unpublished scene is in no book');
        }
    }

    public function testAnUnknownToonAndNoBuildService(): void
    {
        $uses = $this->get(SceneUses::class);
        try {
            $uses->check('nope', null);
            self::fail('an unknown toon');
        } catch (ApiError $e) {
            self::assertSame([404, 'unknown_toon'], [$e->status, $e->errorCode]);
        }
        self::assertSame([], $this->core->requests, 'nothing asked for an unknown toon');
    }

    /** @param array<mixed> $doc */
    private function store(string $key, array $doc): void
    {
        $this->get(EntityStore::class)->insertPublished(EntityKey::parse($key), (string) json_encode($doc, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION), null);
    }
}
