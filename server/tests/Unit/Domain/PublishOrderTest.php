<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use PHPUnit\Framework\TestCase;
use Scutoons\Domain\Catalog\PublishOrder;
use Scutoons\Domain\Content\EntityKey;

/** The order of a bulk publish (#803): what a document names goes first; documents that name each other go together */
final class PublishOrderTest extends TestCase
{
    public function testRefsOfTheRigKinds(): void
    {
        $instrument = ['id' => 'analog_seq', 'graph' => ['nodes' => [['id' => 'seq', 'kind' => 'module:stepseq'], ['id' => 'osc', 'kind' => 'osc']]]];
        self::assertSame(['module:stepseq'], PublishOrder::refs(EntityKey::parse('instrument:analog_seq'), $instrument));
        self::assertSame(['instrument:giantpopstar_theme', 'preset:giantpopstar_theme.base'], PublishOrder::refs(EntityKey::parse('preset:giantpopstar_theme.loud'), ['instrument' => 'giantpopstar_theme', 'parent' => 'giantpopstar_theme.base']));
        $rig = ['subject' => 'character.boy', 'track' => [['instrument' => 'drone', 'preset' => 'drone.dark']], 'moods' => ['sad' => ['presets' => ['drone.sad']]], 'policy' => ['module' => 'boy_rig']];
        self::assertSame(['instrument:drone', 'preset:drone.dark', 'preset:drone.sad', 'module:boy_rig'], PublishOrder::refs(EntityKey::parse('rig:character.boy'), $rig));
        self::assertSame(['rig:character.boy'], PublishOrder::refs(EntityKey::parse('rig-preset:character.boy.soft'), ['rig' => 'character.boy']));
        self::assertSame(['item:spoon'], PublishOrder::refs(EntityKey::parse('recipe:soup'), ['inputs' => [['item' => 'spoon']], 'output' => ['item' => 'spoon']]));
    }

    /** a deck scene names its modules (nodes, slots), its fillers, the rigs they play (pinned, else the filler's own) and rig presets */
    public function testRefsOfAScene(): void
    {
        $scene = [
            'node' => [['id' => 'space', 'kind' => 'space'], ['id' => 'walk', 'kind' => 'module:walkabout']],
            'holder' => [
                ['id' => 'pope', 'ref' => 'character:catpope', 'rig' => ['ref' => 'rig:character.catpope', 'version' => null], 'preset' => 'rig-preset:character.catpope.soft'],
                ['id' => 'boy', 'ref' => 'character:boy', 'module' => [['slot' => 'motion', 'module' => 'dolly']]],
                ['id' => 'who', 'fill' => ['kind' => ['character']]],
                ['id' => 'thing', 'ref' => 'item:incense'],
            ],
        ];
        self::assertSame(
            ['module:walkabout', 'character:catpope', 'rig:character.catpope', 'rig-preset:character.catpope.soft', 'module:dolly', 'character:boy', 'rig:character.boy', 'item:incense', 'rig:item.incense'],
            PublishOrder::refs(EntityKey::parse('scene:nave'), $scene),
        );
    }

    public function testUnitsGoAfterWhatTheyName(): void
    {
        $refs = [
            'rig:character.boy' => ['preset:drone.dark.deep', 'instrument:drone'],
            'preset:drone.dark.deep' => ['preset:drone.dark', 'instrument:drone'],
            'preset:drone.dark' => ['instrument:drone'],
            'instrument:drone' => ['module:walk'],
            'module:walk' => [],
        ];
        $units = PublishOrder::units(['chapter:catpope:1', 'rig:character.boy', 'preset:drone.dark.deep', 'preset:drone.dark', 'instrument:drone', 'module:walk', 'map'], $refs);
        self::assertSame([['module:walk'], ['instrument:drone'], ['preset:drone.dark'], ['preset:drone.dark.deep'], ['rig:character.boy'], ['chapter:catpope:1', 'map']], $units);
    }

    public function testEntriesThatNameEachOtherGoTogether(): void
    {
        $refs = ['item:shirt' => ['product:shirt'], 'product:shirt' => ['item:shirt'], 'recipe:dye' => ['item:shirt']];
        self::assertSame([['item:shirt', 'product:shirt'], ['recipe:dye']], PublishOrder::units(['recipe:dye', 'product:shirt', 'item:shirt'], $refs));
    }

    public function testAModuleGoesLiveOnlyWithACurrentCompile(): void
    {
        $wasm = ['hash' => str_repeat('a', 64), 'bytes' => 10];
        $descriptor = ['abi' => 2, 'name' => 'walk', 'version' => '0.1.0', 'params' => [], 'ports' => []];
        $compiled = ['id' => 'walk', 'files' => ['lib.rs' => 'fn a() {}'], 'descriptor' => $descriptor, 'wasm' => $wasm];
        self::assertSame('not compiled yet – compile it first', PublishOrder::compileProblem(['id' => 'walk', 'files' => ['lib.rs' => '']], null));
        self::assertSame('its sources changed since the last compile – compile it first', PublishOrder::compileProblem(['files' => ['lib.rs' => 'fn b() {}']] + $compiled, $compiled));
        self::assertSame('its build is out of date – compile it first', PublishOrder::compileProblem(['descriptor' => ['abi' => 1]] + $compiled, $compiled));
        self::assertNull(PublishOrder::compileProblem(['removed' => true], null), 'a tombstone needs no build');
    }
}
