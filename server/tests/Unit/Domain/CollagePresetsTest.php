<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Content\CollagePresets;
use stdClass;

/**
 * Custom collage presets baked for readers (agents/research/COLLAGE.md §12.3): CollagePresets::bake gives exactly what
 * @scutoons/content bakePresets gives – both read the golden fixture shared/content/test/fixtures/collage-bake.json –, in
 * both piece forms (stdClass for chapters, assoc arrays for catalog rows); PRESET_ID and STYLE_KEYS equal collage.ts.
 */
final class CollagePresetsTest extends TestCase
{
    private const FIXTURE = __DIR__ . '/../../../../shared/content/test/fixtures/collage-bake.json';
    private const COLLAGE_TS = __DIR__ . '/../../../../shared/content/src/collage.ts';
    private const JSON = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION | JSON_THROW_ON_ERROR;

    private static function fixture(): stdClass
    {
        $fixture = json_decode((string) file_get_contents(self::FIXTURE), false, 512, JSON_THROW_ON_ERROR);
        self::assertInstanceOf(stdClass::class, $fixture);
        return $fixture;
    }

    /** @return array<string, stdClass> */
    private static function index(): array
    {
        return CollagePresets::index((object) ['presets' => self::fixture()->presets]);
    }

    private static function encode(mixed $value): string
    {
        return json_encode($value, self::JSON);
    }

    public function testTheGoldenFixtureInBothPieceForms(): void
    {
        $index = self::index();
        $cases = self::fixture()->cases;
        self::assertIsArray($cases);
        self::assertGreaterThanOrEqual(18, count($cases));
        foreach ($cases as $case) {
            self::assertInstanceOf(stdClass::class, $case);
            $name = (string) $case->name;
            $expected = self::encode($case->baked);
            $piece = $case->piece;
            $unchanged = self::encode($piece) === $expected;
            foreach ([false, true] as $assoc) {
                $form = $assoc ? 'assoc' : 'stdClass';
                $input = json_decode(self::encode($piece), $assoc, 512, JSON_THROW_ON_ERROR);
                $before = self::encode($input);
                $baked = CollagePresets::bake([$input], $index);
                self::assertCount(1, $baked);
                self::assertSame($expected, self::encode($baked[0]), "$name ($form)");
                self::assertSame($before, self::encode($input), "$name ($form): the input is untouched");
                self::assertStringNotContainsString('"style":[]', self::encode($baked[0]), "$name ($form): an emptied style is no key");
                // an unchanged piece is the same value; a baked one names no preset any more
                if ($unchanged) self::assertSame($input, $baked[0], "$name ($form): passed through as it is");
                else self::assertStringNotContainsString('"preset":', self::encode($baked[0]), "$name ($form)");
            }
        }
    }

    public function testTheIndex(): void
    {
        $index = self::index();
        self::assertSame(['night', 'clear', 'dusk', 'lonely-shadow', 'old', 'nothing'], array_keys($index), "PRESET_ID ids only (no 'negative', no 'Night!'), the first of a repeated id, removed ones too");
        self::assertSame('Night', $index['night']->name, 'the first entry wins');
        self::assertTrue($index['old']->removed ?? null);
        foreach ([null, 'x', [], (object) [], (object) ['presets' => 'x'], (object) ['presets' => (object) ['a' => (object) ['id' => 'a']]],
            (object) ['presets' => [null, 3, 'night', ['id' => 'night'], (object) ['name' => 'no id'], (object) ['id' => 7]]]] as $doc) {
            self::assertSame([], CollagePresets::index($doc), self::encode($doc));
        }
        self::assertSame([], CollagePresets::bake([], $index));
        $piece = (object) ['id' => 'piece-a', 'preset' => 'night'];
        self::assertSame([$piece], CollagePresets::bake([$piece], []), 'no presets: the same pieces');
        self::assertSame(['x' => 3, 'y' => 'piece'], CollagePresets::bake(['x' => 3, 'y' => 'piece'], $index), 'no piece objects: as they are, keys kept');
    }

    public function testPresetIdAndStyleKeysEqualCollageTs(): void
    {
        $ts = (string) file_get_contents(self::COLLAGE_TS);
        self::assertSame(1, preg_match('#^export const PRESET_ID = (/\^.*\$/);$#m', $ts, $m));
        self::assertSame(($m[1] ?? '') . 'D', CollagePresets::PRESET_ID, "the same pattern; PHP's D = JS's `\$` (the end only)");
        self::assertSame(1, preg_match('/export const PIECE_STYLE_KEYS = \[(.*?)\] as const;/s', $ts, $k));
        self::assertSame(CollagePresets::STYLE_KEYS, array_map(fn($s) => trim(trim($s), "'"), array_values(array_filter(explode(',', $k[1] ?? ''), fn($s) => trim($s) !== ''))));
        foreach (['night' => 1, 'a' => 1, 'none-at-all' => 1, 'inky' => 1, 'night-2' => 1, str_repeat('a', 40) => 1,
            'negative' => 0, 'none' => 0, 'ink' => 0, 'fade' => 0, 'Night' => 0, '2night' => 0, '-night' => 0, "night\n" => 0, 'night_2' => 0, str_repeat('a', 41) => 0, '' => 0] as $id => $ok) {
            self::assertSame($ok, preg_match(CollagePresets::PRESET_ID, (string) $id), self::encode((string) $id));
        }
    }

    /** groups[].slices[].pieces only, copies where something changes, the input untouched */
    public function testAChapterIsBakedAsACopy(): void
    {
        $index = self::index();
        $raw = [
            'title' => 'One', 'number' => 1,
            'groups' => [
                ['label' => 'vn', 'events' => [['type' => 'showMessage']]],
                ['label' => 'plain', 'slices' => [['image' => 'a.webp', 'pieces' => [['id' => 'piece-a', 'src' => 'v/x.svg', 'preset' => 'ink']]]]],
                ['label' => 'baked', 'slices' => [
                    ['image' => 'b.webp', 'items' => [['id' => 'pickup', 'pieces' => 3, 'preset' => 'night']]],
                    ['image' => 'c.webp', 'pieces' => [
                        ['id' => 'piece-b', 'src' => 'v/x.svg', 'preset' => 'night', 'style' => ['opacity' => 0, 'shadow' => null]],
                        ['id' => 'piece-c', 'src' => 'v/x.svg', 'preset' => 'nope'],
                    ]],
                ]],
                'not a group',
            ],
        ];
        $chapter = json_decode(self::encode($raw), false, 512, JSON_THROW_ON_ERROR);
        self::assertInstanceOf(stdClass::class, $chapter);
        $before = self::encode($chapter);
        $baked = CollagePresets::chapter($chapter, $index);
        self::assertSame($before, self::encode($chapter), 'the input is untouched');
        self::assertNotSame($chapter, $baked);
        $groups = $baked->groups;
        self::assertIsArray($groups);
        self::assertSame($chapter->groups[0], $groups[0], 'a group without slices: the same object');
        self::assertSame($chapter->groups[1], $groups[1], 'nothing to bake: the same object');
        self::assertSame('not a group', $groups[3]);
        self::assertSame($chapter->groups[2]->slices[0], $groups[2]->slices[0], "a pickup's `pieces` is a count, not walked");
        self::assertSame(['id' => 'piece-b', 'src' => 'v/x.svg', 'style' => ['blend' => 'multiply', 'opacity' => 0, 'fill' => '#1a2b6d']],
            json_decode(self::encode($groups[2]->slices[1]->pieces[0]), true));
        self::assertSame($chapter->groups[2]->slices[1]->pieces[1], $groups[2]->slices[1]->pieces[1], 'an unknown id: the same object');
        // the baked style is a copy: changing it leaves the presets' document alone
        $groups[2]->slices[1]->pieces[0]->style->fill = '#ffffff';
        self::assertSame('#1a2b6d', $index['night']->fill);

        self::assertSame($chapter, CollagePresets::chapter($chapter, []), 'no presets: the chapter itself');
        $plain = json_decode(self::encode(['groups' => [$raw['groups'][1]]]), false, 512, JSON_THROW_ON_ERROR);
        self::assertInstanceOf(stdClass::class, $plain);
        self::assertSame($plain, CollagePresets::chapter($plain, $index), 'nothing baked: the chapter itself');
        $none = (object) ['title' => 'no groups'];
        self::assertSame($none, CollagePresets::chapter($none, $index));
    }
}
