<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Library\SceneContent;

/**
 * What a deck's scene needs (#195): sets → their model per level, characters → their portraits per mood. Pure – no
 * database (the HTTP side: LibraryApiTest::testSceneContentForDecks).
 */
final class SceneContentTest extends TestCase
{
    /** @return array<mixed> */
    private static function registry(): array
    {
        $look = fn(string $id, array $moods, bool $default = false) => ['id' => $id] + ($default ? ['default' => true] : [])
            + ['images' => array_map(fn(string $m) => ['avatar' => "a/$id-$m.webp", 'portrait' => "p/$id-$m.webp"], array_combine($moods, $moods))];
        return ['characters' => [
            ['id' => 'catpope', 'personas' => [
                ['id' => 'regular', 'looks' => [$look('default', ['neutral', 'happy'])]],
                ['id' => 'biker', 'looks' => [$look('plain', ['neutral']), $look('helmet', ['neutral', 'dizzy'], true)]],
                ['id' => 'mountaintiger', 'looks' => [['id' => 'default', 'images' => []]]],
            ]],
            ['id' => 'boy', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'images' => ['neutral' => ['avatar' => 'a.webp'], 'sad' => ['portrait' => '']]]]]]],
        ]];
    }

    public function testSetsAnswerTheirModelPerLevel(): void
    {
        $sets = [
            'cathedral' => ['id' => 'cathedral', 'name' => ['en' => 'Cathedral'], 'model' => ['src' => 'r/c.medium.v1.glb', 'variants' => ['low' => 'r/c.low.v1.glb']]],
            'cave' => ['id' => 'cave', 'name' => ['en' => 'Cave'], 'image' => ['src' => '']],
        ];
        $a = SceneContent::compose(['cathedral', 'cave', 'nowhere', 'cathedral'], fn(string $id) => $sets[$id] ?? null, [], []);
        self::assertEquals([
            'low' => ['path' => 'r/c.low.v1.glb', 'size' => null],
            'medium' => ['path' => 'r/c.medium.v1.glb', 'size' => null],
            'high' => ['path' => 'r/c.medium.v1.glb', 'size' => null],
        ], (array) $a['sets']['cathedral']['model'], 'src = medium; a level without a variant gets src');
        self::assertSame(['en' => 'Cathedral'], $a['sets']['cathedral']['name']);
        self::assertSame([], (array) $a['sets']['cave']['model'], 'a set without a model');
        self::assertSame([[], []], [(array) $a['sets']['cave']['still'], (array) $a['sets']['cave']['video']], 'an empty still, no video');
        self::assertSame(['sets' => ['nowhere'], 'locations' => [], 'characters' => []], $a['missing']);
        self::assertSame(['r/c.low.v1.glb', 'r/c.medium.v1.glb'], SceneContent::paths($a));
        $sized = SceneContent::withSizes($a, ['r/c.low.v1.glb' => 2045700]);
        self::assertSame(['path' => 'r/c.low.v1.glb', 'size' => 2045700], ((array) $sized['sets']['cathedral']['model'])['low']);
        self::assertNull(((array) $sized['sets']['cathedral']['model'])['high']['size'], 'unknown to the registry');
    }

    /**
     * #512: a location names its set (one set per location, owner 2026-11-10 [live]); its set answers every rung of the render
     * ladder (SCENES.md §9.4) – the `.glb` per quality, the video loop, the still. The contract with the decks: this answer is
     * fixtures/scenes/scene-content-location.json, which deck-space's push.rs test and the headless scenario 35 read –
     * `UPDATE_FIXTURES=1 npm run dev:db -- test --filter SceneContentTest` writes it, a plain run fails when it is stale.
     */
    public function testALocationNamesItsSetAndTheSetAnswersEveryRung(): void
    {
        $dir = 'catpope/map/sets';
        $sets = ['cathedral' => ['id' => 'cathedral', 'name' => ['de' => 'Kathedrale', 'en' => 'Cathedral'],
            'image' => ['src' => "$dir/cathedral.still.v1.webp", 'variants' => ['high' => "$dir/cathedral.still.4k.v1.webp"]],
            'video' => ['src' => "$dir/cathedral.loop.v1.mp4", 'variants' => ['high' => "$dir/cathedral.loop.4k.v1.mp4"]],
            'model' => ['src' => "$dir/cathedral.medium.v2.glb", 'variants' => ['low' => "$dir/cathedral.low.v2.glb", 'high' => "$dir/cathedral.high.v2.glb"]],
        ]];
        $places = [
            'cathedral' => (object) ['name' => (object) ['de' => 'Kathedrale'], 'toons' => ['catpope'], 'x' => 1, 'y' => 2, 'set' => 'cathedral'],
            'dom' => (object) ['name' => (object) ['de' => 'Dom'], 'toons' => ['catpope'], 'x' => 3, 'y' => 4],
        ];
        $a = SceneContent::compose([], fn(string $id) => $sets[$id] ?? null, [], [], ['cathedral', 'dom', 'nowhere', 'cathedral'], fn(string $id) => $places[$id] ?? null);
        self::assertSame(['cathedral' => ['set' => 'cathedral'], 'dom' => ['set' => null]], $a['locations'], 'a place without a set answers null');
        self::assertSame(['sets' => [], 'locations' => ['nowhere'], 'characters' => []], $a['missing']);
        self::assertSame(['cathedral'], array_keys($a['sets']), "the location's set comes with it");
        $path = fn(string $m, string $level) => ((array) $a['sets']['cathedral'][$m])[$level]['path'];
        self::assertSame(["$dir/cathedral.low.v2.glb", "$dir/cathedral.loop.v1.mp4", "$dir/cathedral.still.v1.webp"], [$path('model', 'low'), $path('video', 'low'), $path('still', 'low')], 'rungs 1, 2, 3 at low');
        self::assertSame(["$dir/cathedral.high.v2.glb", "$dir/cathedral.loop.4k.v1.mp4", "$dir/cathedral.still.4k.v1.webp"], [$path('model', 'high'), $path('video', 'high'), $path('still', 'high')], 'and at high');
        self::assertCount(7, SceneContent::paths($a), 'every file once: 3 glb, 2 videos, 2 stills');
        $sized = SceneContent::withSizes($a, ["$dir/cathedral.loop.v1.mp4" => 5]);
        self::assertSame(5, ((array) $sized['sets']['cathedral']['video'])['medium']['size']);

        $pretty = json_encode(json_decode(SceneContent::json($a)), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION) . "\n";
        $file = dirname(__DIR__, 3) . '/tests/fixtures/scenes/scene-content-location.json';
        if (getenv('UPDATE_FIXTURES')) file_put_contents($file, $pretty);
        self::assertSame(str_replace("\r", '', (string) @file_get_contents($file)), $pretty, 'stale: UPDATE_FIXTURES=1 npm run dev:db -- test --filter SceneContentTest (the decks read it)');
    }

    public function testCharactersResolveIdsLegacyIdsAndPersonas(): void
    {
        $a = SceneContent::compose([], fn() => null, ['catpope', 'plf', 'mountain-tiger', 'catpope-biker', 'catpope-mountaintiger', 'boy', 'catpope-nobody', 'ghost'], self::registry());
        $portraits = fn(string $id) => array_map(fn(array $f) => $f['path'], (array) $a['characters'][$id]['portraits']);
        self::assertSame(['happy' => 'p/default-happy.webp', 'neutral' => 'p/default-neutral.webp'], $portraits('catpope'), 'the regular persona\'s default look, portraits only, by mood');
        self::assertSame(['character' => 'catpope', 'persona' => 'regular', 'look' => 'default'], array_slice($a['characters']['plf'], 0, 3), 'an old VN id: its persona (human) is not in this registry, so the character\'s regular');
        self::assertSame(['character' => 'catpope', 'persona' => 'mountaintiger', 'look' => 'default'], array_slice($a['characters']['mountain-tiger'], 0, 3), 'an old VN id: the persona its art went to (#477, the one table)');
        self::assertSame(['dizzy' => 'p/helmet-dizzy.webp', 'neutral' => 'p/helmet-neutral.webp'], $portraits('catpope-biker'), '<character>-<persona>: its default look');
        self::assertSame([], $portraits('catpope-mountaintiger'), 'a persona without sprites');
        self::assertSame([], $portraits('boy'), 'no portrait files, empty paths skipped');
        self::assertSame(['catpope-nobody', 'ghost'], $a['missing']['characters']);
        self::assertSame('{}', json_encode($a['characters']['boy']['portraits']), 'an empty map stays an object on the wire');
    }

    /** #495: a stage's card shows a character's full view in its mood, an item's picture */
    public function testCardsGetTheFullViewAndTheItemsPicture(): void
    {
        $registry = ['characters' => [['id' => 'catpope', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'images' => [
            'neutral' => ['portrait' => 'p/n.webp', 'full' => 'f/n.webp'],
            'happy' => ['portrait' => 'p/h.webp'],
        ]]]]]]]];
        $a = SceneContent::compose([], fn() => null, ['catpope'], $registry);
        self::assertSame(['neutral' => 'f/n.webp'], array_map(fn(array $f) => $f['path'], (array) $a['characters']['catpope']['full']));
        $a['items'] = ['incense_burner' => ['rig' => ['id' => 'item.incense_burner']]];
        $a['missing']['items'] = ['candle', 'ghost'];
        $catalog = ['incense_burner' => ['id' => 'incense_burner', 'icon' => 'catpope/items/burner.png'], 'candle' => ['id' => 'candle', 'icon' => 'catpope/items/candle.png'], 'rock' => ['id' => 'rock']];
        $a = SceneContent::withItemPictures($a, ['incense_burner', 'candle', 'ghost', 'rock'], fn(string $id) => $catalog[$id] ?? null);
        self::assertSame(['path' => 'catpope/items/burner.png', 'size' => null], $a['items']['incense_burner']['picture'], 'beside its rig');
        self::assertSame(['rig' => null, 'picture' => ['path' => 'catpope/items/candle.png', 'size' => null]], $a['items']['candle'], 'a picture without a rig');
        self::assertSame(['ghost'], $a['missing']['items'], 'known to neither');
        self::assertArrayNotHasKey('rock', $a['items'], 'no rig, no picture: as before');
        self::assertSame(['p/h.webp', 'p/n.webp', 'f/n.webp', 'catpope/items/burner.png', 'catpope/items/candle.png'], SceneContent::paths($a));
        $sized = SceneContent::withSizes($a, ['f/n.webp' => 7, 'catpope/items/candle.png' => 9]);
        self::assertSame(7, ((array) $sized['characters']['catpope']['full'])['neutral']['size']);
        self::assertSame(9, $sized['items']['candle']['picture']['size']);
    }
    /**
     * A look's own form and 3D model reach the deck beside its views: the look's model, else the persona's; a member
     * with neither model nor picture gets both null and empty views (the deck draws its default character, Mellow
     * Mantis). Written to fixtures/scenes/scene-content-characters.json, which deck-space's push.rs test and the
     * headless scenario 32 read – `UPDATE_FIXTURES=1` writes it, a plain run fails when it is stale.
     */
    public function testALooksFormAndModelReachTheDeck(): void
    {
        $views = fn(string $id) => ['neutral' => ['portrait' => "characters/$id/{$id}_portrait-neutral.webp", 'full' => "characters/$id/{$id}_full-neutral.webp"]];
        $registry = ['characters' => [
            ['id' => 'catpope', 'personas' => [['id' => 'regular', 'model' => 'characters/catpope/catpope.glb', 'looks' => [
                ['id' => 'default', 'default' => true, 'form' => 'card', 'model' => 'characters/catpope/catpope-robe.glb', 'images' => $views('catpope')],
            ]]]],
            ['id' => 'toptoy', 'personas' => [['id' => 'regular', 'model' => ' characters/toptoy/toptoy.glb ', 'looks' => [['id' => 'default', 'form' => 'cube', 'images' => $views('toptoy')]]]]],
            ['id' => 'boy', 'personas' => [['id' => 'regular', 'looks' => [['id' => 'default', 'model' => '', 'images' => []]]]]],
            ['id' => 'mellowmantis', 'personas' => [['id' => 'regular', 'model' => 'characters/mellowmantis/mellowmantis.glb', 'looks' => [['id' => 'default', 'images' => $views('mellowmantis')]]]]],
        ]];
        $a = SceneContent::compose([], fn() => null, ['catpope', 'toptoy', 'boy', 'mellowmantis'], $registry);
        $c = $a['characters'];
        self::assertSame(['card', ['path' => 'characters/catpope/catpope-robe.glb', 'size' => null]], [$c['catpope']['form'], $c['catpope']['model']], "the look's form and model win");
        self::assertSame([null, ['path' => 'characters/toptoy/toptoy.glb', 'size' => null]], [$c['toptoy']['form'], $c['toptoy']['model']], "no form word: null; the persona's model");
        self::assertSame([null, null, '{}', '{}'], [$c['boy']['form'], $c['boy']['model'], json_encode($c['boy']['portraits']), json_encode($c['boy']['full'])], 'no art at all');
        self::assertContains('characters/catpope/catpope-robe.glb', SceneContent::paths($a));
        self::assertSame(3, SceneContent::withSizes($a, ['characters/toptoy/toptoy.glb' => 3])['characters']['toptoy']['model']['size']);

        $pretty = json_encode(json_decode(SceneContent::json($a)), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION) . "\n";
        $file = dirname(__DIR__, 3) . '/tests/fixtures/scenes/scene-content-characters.json';
        if (getenv('UPDATE_FIXTURES')) file_put_contents($file, $pretty);
        self::assertSame(str_replace("\r", '', (string) @file_get_contents($file)), $pretty, 'stale: UPDATE_FIXTURES=1 npm run dev:db -- test --filter SceneContentTest (the decks read it)');
    }
}
