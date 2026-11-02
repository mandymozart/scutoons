<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Library\SceneContent;
use Scutoons\Tests\TestCase;

/**
 * scene-content's rigs (#258, agents/research/INSTRUMENTS.md §6): a character's rig (the persona's own, else the
 * character's), item rigs, the instruments and presets with their ancestors, each with a version. The contract with the
 * decks: the answer for the migrated core rigs is server/tests/fixtures/rigs/scene-content.json, which deck-space's
 * the_servers_scene_content_reads_as_bundles test reads as `rig::Bundle`s and compares with deck/native/rigs/*.toml –
 * `UPDATE_FIXTURES=1 npm run dev:db -- test --filter SceneContentRigs` writes it, a plain run fails when it is stale.
 */
final class SceneContentRigsTest extends TestCase
{
    private const FIXTURE = '/tests/fixtures/rigs/scene-content.json';

    /** @return array<mixed> the character registry: catpope with a biker persona, a boy without a rig */
    private static function registry(): array
    {
        $look = ['id' => 'default', 'default' => true, 'images' => ['neutral' => ['portrait' => 'characters/catpope/catpope_portrait-neutral.webp']]];
        return ['characters' => [
            ['id' => 'catpope', 'personas' => [['id' => 'regular', 'looks' => [$look]], ['id' => 'biker', 'looks' => [$look]]]],
            ['id' => 'boy', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'images' => []]]]]],
        ]];
    }

    /** a catalog row's stored data ('' when there is none) */
    private function stored(string $kind, string $id): string
    {
        return $this->get(CatalogStore::class)->row($kind, $id)['data'] ?? '';
    }

    public function testTheCoreRigsAnswerAsTheDecksReadThem(): void
    {
        $store = $this->get(CatalogStore::class);
        $answer = SceneContent::compose([], fn() => null, ['catpope', 'boy'], self::registry());
        $json = SceneContent::json(SceneContent::withRigs($answer, ['incense_burner', 'ghost'], fn(string $kind, string $id) => $store->row($kind, $id)['data'] ?? null));
        $pretty = json_encode(json_decode($json), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION) . "\n";
        $path = self::root() . self::FIXTURE;
        if (getenv('UPDATE_FIXTURES')) {
            @mkdir(dirname($path), 0777, true);
            file_put_contents($path, $pretty);
        }
        self::assertSame(str_replace("\r", '', (string) @file_get_contents($path)), $pretty, 'stale: UPDATE_FIXTURES=1 npm run dev:db -- test --filter SceneContentRigs (deck-space\'s contract test reads it)');

        $a = json_decode($json, true);
        $rig = $a['characters']['catpope']['rig'];
        self::assertSame('character.catpope', $rig['id']);
        self::assertSame(['cathedral_voice', 'drone'], $rig['instruments']);
        self::assertSame(['cathedral_voice.base', 'drone.base', 'cathedral_voice.bright', 'cathedral_voice.low'], $rig['presets'], 'every preset it names (tracks, then the moods in order), ancestors first');
        self::assertSame(SceneContent::version($this->stored('rigs', 'character.catpope')), $rig['version']);
        self::assertNull($a['characters']['boy']['rig'], 'a character without a rig');
        self::assertSame(['incense'], $a['items']['incense_burner']['rig']['instruments']);
        self::assertSame(['items' => ['ghost']], array_intersect_key($a['missing'], ['items' => 1]));
        self::assertSame(['cathedral_voice', 'drone', 'incense'], array_keys($a['instruments']));
        self::assertSame(['id' => 'drone.base', 'version' => SceneContent::version($this->stored('presets', 'drone.base'))], array_slice($a['presets']['drone.base'], 0, 2));
        self::assertStringContainsString('"freq":147.0', $json, '1.0 stays 1.0 – a node param stays a float in TOML');
    }

    public function testAPersonasOwnRigWinsAndAncestorsComeAlong(): void
    {
        $rows = [
            'rigs' => [
                'character.catpope' => '{"subject":"character.catpope","track":[{"id":"v","instrument":"x","preset":"x.a"}]}',
                'character.catpope.biker' => '{"subject":"character.catpope.biker","track":[{"id":"v","instrument":"x","preset":"x.c"}],"moods":{"sad":{}}}',
            ],
            'presets' => [
                'x.a' => '{"id":"x.a","instrument":"x"}',
                'x.b' => '{"id":"x.b","instrument":"x","parent":"x.a"}',
                'x.c' => '{"id":"x.c","instrument":"x","parent":"x.b"}',
            ],
            'instruments' => ['x' => '{"id":"x","kind":"pad","node":[],"out":"o"}'],
        ];
        $answer = SceneContent::compose([], fn() => null, ['catpope', 'catpope-biker'], self::registry());
        $a = json_decode(SceneContent::json(SceneContent::withRigs($answer, [], fn(string $kind, string $id) => $rows[$kind][$id] ?? null)), true);
        self::assertSame('character.catpope', $a['characters']['catpope']['rig']['id']);
        self::assertSame('character.catpope.biker', $a['characters']['catpope-biker']['rig']['id'], 'the persona\'s own rig');
        self::assertSame(['x.a', 'x.b', 'x.c'], $a['characters']['catpope-biker']['rig']['presets'], 'with its ancestors, root first');
        self::assertSame(['sad' => []], $a['characters']['catpope-biker']['rig']['data']['moods']);
        self::assertStringContainsString('"moods":{"sad":{}}', SceneContent::json(SceneContent::withRigs($answer, [], fn(string $kind, string $id) => $rows[$kind][$id] ?? null)), '{} stays an object (serde\'s Variant)');
        self::assertSame(['x.a', 'x.b', 'x.c'], array_keys($a['presets']));
    }

    public function testNoRigsAnswerEmptyMaps(): void
    {
        $a = SceneContent::json(SceneContent::withRigs(SceneContent::compose([], fn() => null, [], []), [], fn() => null));
        self::assertSame('{"sets":{},"characters":{},"items":{},"instruments":{},"presets":{},"missing":{"sets":[],"characters":[],"items":[],"presets":[]}}', $a);
    }
}
