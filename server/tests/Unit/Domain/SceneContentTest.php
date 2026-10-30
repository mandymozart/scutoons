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
        $a = SceneContent::compose([], fn() => null, ['catpope', 'plf', 'catpope-biker', 'catpope-mountaintiger', 'boy', 'catpope-nobody', 'ghost'], self::registry());
        $portraits = fn(string $id) => array_map(fn(array $f) => $f['path'], (array) $a['characters'][$id]['portraits']);
        self::assertSame(['happy' => 'p/default-happy.webp', 'neutral' => 'p/default-neutral.webp'], $portraits('catpope'), 'the regular persona\'s default look, portraits only, by mood');
        self::assertSame(['character' => 'catpope', 'persona' => 'regular', 'look' => 'default'], array_slice($a['characters']['plf'], 0, 3), 'an old VN id');
        self::assertSame(['dizzy' => 'p/helmet-dizzy.webp', 'neutral' => 'p/helmet-neutral.webp'], $portraits('catpope-biker'), '<character>-<persona>: its default look');
        self::assertSame([], $portraits('catpope-mountaintiger'), 'a persona without sprites');
        self::assertSame([], $portraits('boy'), 'no portrait files, empty paths skipped');
        self::assertSame(['catpope-nobody', 'ghost'], $a['missing']['characters']);
        self::assertSame('{}', json_encode($a['characters']['boy']['portraits']), 'an empty map stays an object on the wire');
    }
}
