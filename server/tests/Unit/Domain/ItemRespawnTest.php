<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\ItemRespawn;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/** Placed items that reappear (agents/ITEMS.md "Placed items" → "Respawn"): the item's rule, admin overrides, grants per period */
final class ItemRespawnTest extends TestCase
{
    private ItemRespawn $respawn;
    private InventoryService $inventory;

    protected function setUp(): void
    {
        parent::setUp();
        $this->respawn = $this->get(ItemRespawn::class);
        $this->inventory = $this->get(InventoryService::class);
    }

    public function testIntervalsAndRules(): void
    {
        self::assertSame(0, ItemRespawn::minutes('visit'));
        self::assertSame(90, ItemRespawn::minutes(['minutes' => 90]));
        self::assertSame(1440, ItemRespawn::minutes(['days' => 1]));
        foreach (['day', ['hours' => 0], ['hours' => 1.5], ['hours' => 1, 'days' => 1], ['weeks' => 1], ['days' => 400]] as $bad) {
            self::assertNull(ItemRespawn::minutes($bad), (string) json_encode($bad));
        }
        self::assertSame([], ItemRespawn::problems(null));
        self::assertSame([], ItemRespawn::problems(['default' => ['hours' => 6], 'min' => ['hours' => 1], 'max' => ['days' => 7]]));
        self::assertNotSame([], ItemRespawn::problems(['default' => ['hours' => 1], 'min' => ['hours' => 2]]));
        // config/items.json: cucumber 1 h … 7 d (default 6 h); the record player never reappears
        self::assertSame(['default' => ['hours' => 6], 'min' => ['hours' => 1], 'max' => ['days' => 7]], $this->respawn->rule('cucumber'));
        self::assertNull($this->respawn->rule('broken_record_player'));
    }

    public function testAGrantPerPeriodWithinTheItemsRange(): void
    {
        $id = (string) $this->user()['id'];
        $grant = fn(mixed $respawn, ?string $visit = null) => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'vn:cucumber:/sandbox/chapter/2#s1.webp#cucumber-1', 'respawn' => $respawn, 'visit' => $visit]);
        self::assertSame(1, $grant(['hours' => 2]));
        self::assertSame(0, $grant(['hours' => 2]), 'once per period');
        $this->clock->advance(2 * 3600);
        self::assertSame(1, $grant(['hours' => 2]), 'the next period');
        $this->assertError('respawn_out_of_range', fn() => $grant(['minutes' => 5]));
        $this->assertError('respawn_out_of_range', fn() => $grant('visit'), 'every visit is shorter than the hour the item allows');
        $this->assertError('respawn_out_of_range', fn() => $grant(['days' => 30]));
        $this->assertError('invalid_respawn', fn() => $grant('daily'));
        $this->assertError('respawn_not_allowed', fn() => $this->inventory->grant($id, ['item_id' => 'repair_coupon', 'source' => 'story', 'ref' => 'x', 'respawn' => ['hours' => 1]]));
        $this->assertError('ref_required', fn() => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story', 'respawn' => ['hours' => 1]]));
    }

    public function testAdminOverridesTheItemsRule(): void
    {
        $id = (string) $this->user()['id'];
        self::assertTrue($this->respawn->update('cucumber', ['default' => 'visit', 'min' => 'visit', 'max' => ['days' => 1]], false, 'admin:a@b.c'));
        self::assertFalse($this->respawn->update('cucumber', ['default' => 'visit', 'min' => 'visit', 'max' => ['days' => 1]], false, 'admin:a@b.c'), 'unchanged');
        $grant = fn(?string $visit) => $this->inventory->grant($id, ['item_id' => 'cucumber', 'source' => 'story', 'ref' => 'place', 'respawn' => 'visit', 'visit' => $visit]);
        self::assertSame(1, $grant('visit1'));
        self::assertSame(0, $grant('visit1'), 'once per visit');
        self::assertSame(1, $grant('visit2'));
        $this->assertError('visit_required', fn() => $grant(null));
        // the served catalog carries the override; null = never
        $served = array_column($this->respawn->apply($this->get(\Scutoons\Domain\Catalog\Catalog::class)->data())['items'], null, 'id');
        self::assertSame('visit', $served['cucumber']['respawn']['default']);
        $this->respawn->update('cucumber', null, false, 'admin:a@b.c');
        self::assertNull($this->respawn->rule('cucumber'));
        $this->respawn->update('cucumber', null, true, 'admin:a@b.c');
        self::assertSame(['hours' => 6], $this->respawn->rule('cucumber')['default'] ?? null, 'reset → the file');
        $this->assertError('invalid_respawn', fn() => $this->respawn->update('cucumber', ['default' => 'x'], false, 'admin:a@b.c'));
        $this->assertError('unknown_item', fn() => $this->respawn->update('nope', null, false, 'admin:a@b.c'));
        $state = $this->respawn->state();
        self::assertSame('admin:a@b.c', $state['updated_by']);
    }

    private function assertError(string $code, callable $call, string $message = ''): void
    {
        try {
            $call();
            self::fail("expected $code $message");
        } catch (ApiError $error) {
            self::assertSame($code, $error->errorCode, $message);
        }
    }
}
