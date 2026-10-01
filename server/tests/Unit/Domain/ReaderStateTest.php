<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Domain\Reader\ReaderState;
use Scutoons\Tests\TestCase;

/** agents/ITEMS.md "Substances": testo and the other substances, stats with decay, timed effects, the testo camera */
final class ReaderStateTest extends TestCase
{
    /** @param callable(): mixed $call */
    private function error(string $code, callable $call): void
    {
        try {
            $call();
            self::fail("expected $code");
        } catch (ApiError $e) {
            self::assertSame($code, $e->errorCode);
        }
    }

    public function testDecayMath(): void
    {
        $t = 1_800_000_000;
        self::assertSame(50.0, ReaderState::level(50, 3, null, null, $t), 'nothing stored = the baseline');
        self::assertSame(90.0, ReaderState::level(50, 3, 90, $t, $t));
        self::assertEqualsWithDelta(70.0, ReaderState::level(50, 3, 90, $t, $t + 3 * 86400), 1e-9, 'one half-life: half the distance');
        self::assertEqualsWithDelta(60.0, ReaderState::level(50, 3, 90, $t, $t + 6 * 86400), 1e-9);
        self::assertEqualsWithDelta(40.0, ReaderState::level(50, 3, 30, $t, $t + 3 * 86400), 1e-9, 'from below too');
    }

    public function testApplyTestoRaisesCapsAndDecays(): void
    {
        $id = (string) $this->user()['id'];
        $inventory = $this->get(InventoryService::class);
        $reader = $this->get(ReaderState::class);
        self::assertSame(['level' => 50.0, 'updated_at' => gmdate('c', $this->clock->time())], $reader->stats($id)['testo'], 'the neutral start, as of now');
        $this->error('missing', fn() => $reader->use($id, ['item_id' => 'testo']));
        $inventory->add($id, 'testo', null, 4, 'story');
        $reader->use($id, ['item_id' => 'testo']);
        self::assertSame(70.0, $reader->stats($id)['testo']['level']);
        self::assertSame(3, $inventory->count($id, 'testo'), 'one piece used up');
        $reader->use($id, ['item_id' => 'testo']);
        $reader->use($id, ['item_id' => 'testo']);
        self::assertSame(100.0, $reader->stats($id)['testo']['level'], 'capped at 100');
        $this->clock->advance(3 * 86400);
        self::assertSame(75.0, $reader->stats($id)['testo']['level'], 'three days later (one half-life): halfway back to 50');
        $reader->use($id, ['item_id' => 'testo']);
        self::assertSame(95.0, $reader->stats($id)['testo']['level'], 'applied on the decayed level');
        $this->error('not_usable', fn() => $reader->use($id, ['item_id' => 'camera']));
        $this->error('unknown_item', fn() => $reader->use($id, ['item_id' => 'nope']));
        // the other stat: semaglutide lowers appetite
        $inventory->add($id, 'semaglutide', null, 1, 'story');
        $reader->use($id, ['item_id' => 'semaglutide']);
        self::assertSame(30.0, $reader->stats($id)['appetite']['level']);
        // GET /me/ carries both
        $me = $this->get(AccountService::class)->me($this->db->one('SELECT * FROM users WHERE id = ?', [$id]) ?? []);
        self::assertSame([95.0, 30.0], [((array) $me['stats'])['testo']['level'], ((array) $me['stats'])['appetite']['level']]);
    }

    public function testWeedRaisesAppetiteEatingLowersIt(): void
    {
        $id = (string) $this->user()['id'];
        $inventory = $this->get(InventoryService::class);
        $reader = $this->get(ReaderState::class);
        $inventory->add($id, 'weed', null, 3, 'story');
        $reader->use($id, ['item_id' => 'weed']);
        self::assertSame(75.0, $reader->stats($id)['appetite']['level'], 'weed: +25');
        self::assertSame([], $reader->effects($id), 'no reader effect');
        $reader->use($id, ['item_id' => 'weed']);
        $reader->use($id, ['item_id' => 'weed']);
        self::assertSame(100.0, $reader->stats($id)['appetite']['level'], 'capped at 100');
        $inventory->add($id, 'cucumber', null, 1, 'story');
        $reader->use($id, ['item_id' => 'cucumber']);
        self::assertSame(95.0, $reader->stats($id)['appetite']['level'], 'eat a cucumber: −5');
        self::assertSame(0, $inventory->count($id, 'cucumber'), 'the cucumber is eaten');
        $inventory->add($id, 'tuna_can', null, 4, 'story');
        $reader->use($id, ['item_id' => 'tuna_can']);
        self::assertSame(65.0, $reader->stats($id)['appetite']['level'], 'eat a can of tuna: −30');
        for ($i = 0; $i < 3; $i++) $reader->use($id, ['item_id' => 'tuna_can']);
        self::assertSame(0.0, $reader->stats($id)['appetite']['level'], 'floored at 0');
        $this->clock->advance(3 * 86400);
        self::assertSame(25.0, $reader->stats($id)['appetite']['level'], 'and it drifts back toward 50');
    }

    public function testTimedEffectExpiresAndCanBeEnded(): void
    {
        $id = (string) $this->user()['id'];
        $this->get(InventoryService::class)->add($id, 'psilocybin', null, 2, 'story');
        $reader = $this->get(ReaderState::class);
        $reader->use($id, ['item_id' => 'psilocybin']);
        $effects = $reader->effects($id);
        self::assertSame('psychedelic', $effects[0]['kind'] ?? null);
        self::assertSame(gmdate('c', $this->clock->time() + 20 * 60), $effects[0]['until']);
        $this->clock->advance(19 * 60);
        self::assertCount(1, $reader->effects($id));
        $this->clock->advance(2 * 60);
        self::assertSame([], $reader->effects($id), 'expired after 20 minutes');
        $reader->use($id, ['item_id' => 'psilocybin']);
        self::assertTrue($reader->end($id, 'psychedelic'));
        self::assertSame([], $reader->effects($id), '"Effekt beenden"');
        self::assertFalse($reader->end($id, 'psychedelic'));
    }

    public function testTestoCameraRecipeAndScanCooldown(): void
    {
        $id = (string) $this->user()['id'];
        $inventory = $this->get(InventoryService::class);
        $this->error('no_tool', fn() => $inventory->grant($id, ['item_id' => 'testo', 'source' => 'scan']));
        $inventory->add($id, 'testo', null, 1, 'story');
        $inventory->craft($id, 'testo_camera');
        self::assertSame([0, 0, 1], [$inventory->count($id, 'camera'), $inventory->count($id, 'testo'), $inventory->count($id, 'testo_camera')]);
        self::assertSame(1, $inventory->grant($id, ['item_id' => 'testo', 'source' => 'scan']));
        $this->error('cooldown', fn() => $inventory->grant($id, ['item_id' => 'testo', 'source' => 'scan']));
        $this->clock->advance(11);
        self::assertSame(1, $inventory->grant($id, ['item_id' => 'testo', 'source' => 'scan']));
        self::assertSame(2, $inventory->count($id, 'testo'));
        // the Check It camera (story-granted): its map grants the other substances by scan, each with the cooldown
        $this->error('no_tool', fn() => $inventory->grant($id, ['item_id' => 'semaglutide', 'source' => 'scan']));
        $inventory->grant($id, ['item_id' => 'checkit_camera', 'source' => 'story', 'ref' => 'test:checkit']);
        foreach (['semaglutide', 'psilocybin', 'lsd'] as $item) self::assertSame(1, $inventory->grant($id, ['item_id' => $item, 'source' => 'scan']), $item);
        $this->error('cooldown', fn() => $inventory->grant($id, ['item_id' => 'psilocybin', 'source' => 'scan']));
        // the Check It camera's recipe (owner 2026-10-01): camera + testo, like the testo camera
        $inventory->add($id, 'camera', null, 1, 'starter');
        $inventory->add($id, 'testo', null, 1, 'story');
        $inventory->craft($id, 'checkit_camera');
        self::assertSame(2, $inventory->count($id, 'checkit_camera'), 'one from the story grant, one crafted');
        // a story pickup (the sandbox's testo) is granted once per place
        self::assertSame(1, $inventory->grant($id, ['item_id' => 'testo', 'source' => 'story', 'ref' => 'sandbox:testo']));
        self::assertSame(0, $inventory->grant($id, ['item_id' => 'testo', 'source' => 'story', 'ref' => 'sandbox:testo']));
    }
}
