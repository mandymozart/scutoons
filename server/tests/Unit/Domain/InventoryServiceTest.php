<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Progress\ProgressService;
use Scutoons\Tests\TestCase;

final class InventoryServiceTest extends TestCase
{
    private InventoryService $inventory;

    protected function setUp(): void
    {
        parent::setUp();
        $this->inventory = $this->get(InventoryService::class);
    }

    public function testStackableItemsShareOneRowOthersArePieces(): void
    {
        $id = (string) $this->user()['id'];
        $this->inventory->add($id, 'cucumber', null, 2, 'story');
        $this->inventory->add($id, 'cucumber', null, 1, 'scan');
        $this->inventory->add($id, 'camera', null, 2, 'story');
        self::assertSame(3, $this->inventory->count($id, 'cucumber'));
        self::assertSame(3, $this->inventory->count($id, 'camera'), 'starter + 2 pieces');
        self::assertCount(1, $this->db->all("SELECT id FROM inventory WHERE user_id = ? AND item_id = 'cucumber'", [$id]));
        $this->expectException(ApiError::class);
        $this->inventory->add($id, 'unicorn', null, 1, 'story');
    }

    public function testCraftConsumesInputsAndReportsWhatIsMissing(): void
    {
        $id = (string) $this->user()['id'];
        try {
            $this->inventory->craft($id, 'cucumber_camera');
            self::fail('missing inputs expected');
        } catch (ApiError $error) {
            self::assertSame('missing_inputs', $error->errorCode);
            self::assertSame([['item' => 'cucumber', 'qty' => 1]], $error->extra['missing']);
        }
        $this->inventory->add($id, 'cucumber', null, 1, 'story');
        self::assertSame('cucumber_camera', $this->inventory->craft($id, 'cucumber_camera')['id']);
        self::assertSame(0, $this->inventory->count($id, 'camera'));
        self::assertSame(1, $this->inventory->count($id, 'cucumber_camera'));
    }

    public function testGrantRespectsSourceRefToolAndCooldown(): void
    {
        $id = (string) $this->user()['id'];
        self::assertSame(1, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'ch1:s3']));
        self::assertSame(0, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'ch1:s3']), 'same ref grants nothing');
        $this->assertError('not_grantable', fn() => $this->inventory->grant($id, ['item_id' => 'camera', 'source' => 'story']));
        $this->assertError('invalid_source', fn() => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'shop']));
        $this->assertError('no_tool', fn() => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']));
        $this->inventory->craft($id, 'cucumber_camera');
        self::assertSame(1, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']));
        $this->assertError('cooldown', fn() => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']));
    }

    public function testScanGrantsStopAtTheCamerasDailyLimit(): void
    {
        $id = (string) $this->user()['id'];
        $this->inventory->add($id, 'cucumber', null, 1, 'story');
        $this->inventory->craft($id, 'cucumber_camera');
        for ($i = 0; $i < 3; $i++) {
            self::assertSame(1, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']));
            $this->clock->advance(60);
        }
        $this->assertError('daily_limit', fn() => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']));
        self::assertSame(1, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story']), 'story finds are not scans');
        $this->clock->advance(86400);
        self::assertSame(1, $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'scan']), 'a day later');
    }

    public function testImportTakesOnlyGrantableItems(): void
    {
        $id = (string) $this->user()['id'];
        $result = $this->inventory->import($id, ['items' => [['item_id' => 'cucumber', 'qty' => 500], ['item_id' => 'dago_hoodie', 'qty' => 1], ['item_id' => 'nope']]]);
        self::assertSame([['item_id' => 'cucumber', 'qty' => 99]], $result['imported']);
        self::assertSame(['not_grantable', 'unknown_item'], array_column($result['skipped'], 'reason'));
        $this->assertError('invalid_items', fn() => $this->inventory->import($id, ['items' => 'x']));
    }

    public function testAvatarSlotsAndCraftingUnequips(): void
    {
        $user = $this->user();
        $id = (string) $user['id'];
        $avatar = $this->get(AvatarService::class);
        // a new account wears the basic dress (agents/ITEMS.md "Wardrobe"): T-shirt, shorts, sandals
        $wear = fn(string $item) => (string) $this->db->value('SELECT id FROM inventory WHERE user_id = ? AND item_id = ?', [$id, $item]);
        self::assertSame('{"head":null,"face":null,"neck":null,"torso":"' . $wear('scutoons_tshirt') . '","hands":null,"legs":"' . $wear('scutoons_shorts') . '","feet":"' . $wear('scutoons_sandals') . '","back":null,"accessory":null}',
            json_encode($avatar->toJson($id)['slots']));
        $this->inventory->add($id, 'dago_hoodie', 'black', 1, 'story');
        $hoodie = (string) $this->db->value("SELECT id FROM inventory WHERE user_id = ? AND item_id = 'dago_hoodie'", [$id]);
        $avatar->update($id, ['slots' => ['torso' => $hoodie]]);
        self::assertSame($hoodie, ((array) $avatar->toJson($id)['slots'])['torso']);
        $this->assertError('wrong_slot', fn() => $avatar->update($id, ['slots' => ['head' => $hoodie]]));
        $this->assertError('invalid_slot', fn() => $avatar->update($id, ['slots' => ['tail' => null]]));
        $this->assertError('not_owned', fn() => $avatar->update((string) $this->user()['id'], ['slots' => ['torso' => $hoodie]]));
        $this->assertError('invalid_slots', fn() => $avatar->update($id, ['slots' => ['x']]));
        $avatar->update($id, ['slots' => ['torso' => null]]);
        self::assertNull(((array) $avatar->toJson($id)['slots'])['torso']);
    }

    public function testProgressLastWriteWins(): void
    {
        $id = (string) $this->user()['id'];
        $progress = $this->get(ProgressService::class);
        self::assertSame(['toon_id' => 'catpope', 'state' => null, 'updated_at' => null], $progress->get($id, 'catpope'));
        self::assertSame('success', $progress->put($id, 'catpope', (object) ['a' => new \stdClass()], 2000)['status']);
        $stale = $progress->put($id, 'catpope', (object) ['a' => 1], 1000);
        self::assertSame(['stale', 2000], [$stale['status'], $stale['updated_at']]);
        self::assertSame('{"a":{}}', json_encode($progress->get($id, 'catpope')['state']), '{} stays an object');
        $this->assertError('invalid_state', fn() => $progress->put($id, 'catpope', 'x', 1));
        $this->assertError('invalid_updated_at', fn() => $progress->put($id, 'catpope', [], -1));
    }

    /** The world state `scutoons` (agents/MAP.md §2.4): map discoveries merge as sets, a stale write loses none */
    public function testWorldProgressMergesTheMapState(): void
    {
        $id = (string) $this->user()['id'];
        $progress = $this->get(ProgressService::class);
        $state = fn(array $map, string $note) => json_decode((string) json_encode(['map' => $map, 'note' => $note]));
        $progress->put($id, 'scutoons', $state(['places' => ['home' => ['unlocked' => 100, 'seen' => 150]], 'routes' => ['way' => ['revealed' => 1]],
            'positions' => ['t' => ['location' => 'home', 'at' => 100]]], 'phone'), 2000);
        // a laptop that never saw the phone's state writes later: union, not replace
        $r = $progress->put($id, 'scutoons', $state(['places' => ['tower' => ['unlocked' => 300], 'home' => ['unlocked' => 50]], 'routes' => ['way' => ['revealed' => 0, 'travelled' => 0]],
            'positions' => ['t' => ['location' => 'tower', 'at' => 300]]], 'laptop'), 3000);
        self::assertSame('success', $r['status']);
        $map = json_decode((string) json_encode($r['state']->map), true);
        self::assertEquals(['home' => ['unlocked' => 50, 'seen' => 150], 'tower' => ['unlocked' => 300]], $map['places']);
        self::assertSame(['way' => ['revealed' => 1, 'travelled' => 0]], $map['routes']);
        self::assertSame('tower', $map['positions']['t']['location']);
        self::assertSame('laptop', $r['state']->note, 'other fields: the newer write');
        // an older write (stale) still adds its discoveries; the newer fields stay
        $r = $progress->put($id, 'scutoons', $state(['places' => ['pond' => ['unlocked' => 10]], 'positions' => ['t' => ['location' => 'pond', 'at' => 10]]], 'old'), 1000);
        self::assertSame(['stale', 3000], [$r['status'], $r['updated_at']]);
        $stored = json_decode((string) json_encode($progress->get($id, 'scutoons')['state']), true);
        self::assertEqualsCanonicalizing(['home', 'tower', 'pond'], array_keys($stored['map']['places']));
        self::assertSame(['tower', 'laptop'], [$stored['map']['positions']['t']['location'], $stored['note']]);
        // other ids stay last-write-wins
        $progress->put($id, 'catpope', $state(['places' => ['a' => ['unlocked' => 1]]], 'x'), 2000);
        $r = $progress->put($id, 'catpope', $state(['places' => ['b' => ['unlocked' => 1]]], 'y'), 1000);
        self::assertSame(['a'], array_keys((array) $r['state']->map->places));
    }

    /** @param callable(): mixed $call */
    private function assertError(string $code, callable $call): void
    {
        try {
            $call();
            self::fail("expected $code");
        } catch (ApiError $error) {
            self::assertSame($code, $error->errorCode);
        }
    }
}
