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
        self::assertSame(['sets' => ['nowhere'], 'characters' => []], $a['missing']);
        self::assertSame(['r/c.low.v1.glb', 'r/c.medium.v1.glb'], SceneContent::paths($a));
        $sized = SceneContent::withSizes($a, ['r/c.low.v1.glb' => 2045700]);
        self::assertSame(['path' => 'r/c.low.v1.glb', 'size' => 2045700], ((array) $sized['sets']['cathedral']['model'])['low']);
        self::assertNull(((array) $sized['sets']['cathedral']['model'])['high']['size'], 'unknown to the registry');
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
}
