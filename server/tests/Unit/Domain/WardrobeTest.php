<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/**
 * agents/ITEMS.md "Wardrobe": the default wearables, the basic dress (T-shirt, shorts, sandals – the swim pants on a body
 * without a shorts overlay), swap, the cover rule while "Hide sensitive content" is on
 */
final class WardrobeTest extends TestCase
{
    private function registry(): void
    {
        $slot = fn(string $id, string $type) => ['id' => $id, 'slot' => $type, 'x' => 1, 'y' => 1, 'size' => 1];
        $single = array_map(fn(string $t) => $slot($t, $t), ['head', 'face', 'neck', 'torso', 'hands', 'legs', 'feet', 'back', 'accessory']);
        $twins = [$slot('torso-left', 'torso'), $slot('torso-right', 'torso'), $slot('legs-left', 'legs'), $slot('legs-right', 'legs'), $slot('feet-left', 'feet'), $slot('feet-right', 'feet')];
        $looks = fn(string $c, array $slots) => ['looks' => [
            ['id' => 'swim', 'default' => true, 'path' => "characters/$c/{$c}_apose-swim.webp", 'slots' => $slots],
            ['id' => 'naked', 'sensitive' => true, 'path' => "characters/$c/{$c}_apose-naked.webp", 'slots' => $slots],
        ]];
        $json = json_encode(['characters' => [['id' => 'catpope', 'apose' => $looks('catpope', $single)], ['id' => 'toptoy', 'apose' => $looks('toptoy', $twins)]]]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, '2026-10-01 00:00:00', 'published', NULL)", [$json]);
    }

    /** @return array<string, string> item id → entry id */
    private function entries(string $user): array
    {
        $out = [];
        foreach ($this->db->all('SELECT id, item_id FROM inventory WHERE user_id = ? ORDER BY acquired_at, id', [$user]) as $r) $out[$r['item_id']] ??= (string) $r['id'];
        return $out;
    }

    /** @return array<string, string|null> */
    private function slots(string $user): array
    {
        return (array) $this->get(AvatarService::class)->toJson($user)['slots'];
    }

    public function testANewAccountWearsTheBasicDress(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $e = $this->entries($user);
        self::assertArrayHasKey('scutoons_swim_pants', $e, 'the swim pants are a starter too (in the inventory, not worn)');
        $slots = $this->slots($user);
        self::assertSame([$e['scutoons_tshirt'], $e['scutoons_shorts'], $e['scutoons_sandals']], [$slots['torso'], $slots['legs'], $slots['feet']]);
    }

    public function testTheBasicDressPerBody(): void
    {
        $avatar = $this->get(AvatarService::class);
        $ids = fn(?string $c) => array_column($avatar->basicDress($c), 'id');
        self::assertSame(['scutoons_tshirt', 'scutoons_shorts', 'scutoons_sandals'], $ids('catpope'));
        self::assertSame(['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_sandals'], $ids('yellowmouse'), 'no shorts overlay: the swim pants');
        self::assertSame(['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_sandals'], $ids('toptoy'));
        self::assertSame(['scutoons_tshirt', 'scutoons_shorts', 'scutoons_sandals'], $ids(null));
    }

    public function testOlderAccountsGetTheDefaultWearablesOnce(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $this->db->run("DELETE FROM avatar_slots WHERE user_id = ?", [$user]);
        $this->db->run("DELETE FROM inventory WHERE user_id = ? AND item_id LIKE 'scutoons_%'", [$user]);
        $avatar = $this->get(AvatarService::class);
        self::assertSame(['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_shorts', 'scutoons_sandals'], $avatar->ensureWardrobe($user));
        self::assertSame([], $avatar->ensureWardrobe($user), 'idempotent');
        $me = $this->get(AccountService::class)->me($this->db->one('SELECT * FROM users WHERE id = ?', [$user]) ?? []);
        self::assertCount(1, array_filter($me['inventory'], fn($x) => $x['item_id'] === 'scutoons_sandals'));
        $e = $this->entries($user);
        self::assertSame([$e['scutoons_shorts'], $e['scutoons_sandals']], [$this->slots($user)['legs'], $this->slots($user)['feet']]);
    }

    public function testOnTheTwinsTheOlderAccountGetsTheSwimPants(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $this->get(CharacterUnlocks::class)->grant($user, 'toptoy', [], 'admin', null);
        $avatar = $this->get(AvatarService::class);
        $avatar->update($user, ['body' => ['character' => 'toptoy', 'look' => 'swim']]);
        $this->db->run("DELETE FROM avatar_slots WHERE user_id = ?", [$user]);
        $this->db->run("DELETE FROM inventory WHERE user_id = ? AND item_id LIKE 'scutoons_%'", [$user]);
        $avatar->ensureWardrobe($user);
        $e = $this->entries($user);
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs-left'], 'Top Toy has no shorts overlay: the swim pants');
    }

    public function testSwapKeepsTheOldItemAndTheCoverRuleRefillsLegs(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $e = $this->entries($user);
        $avatar = $this->get(AvatarService::class);

        // swim pants on → the shorts come off but stay in the inventory
        self::assertSame([], $avatar->update($user, ['slots' => ['legs' => $e['scutoons_swim_pants']]]));
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs']);
        self::assertSame(1, $this->get(InventoryService::class)->count($user, 'scutoons_shorts'));
        // hidden (default): the swim pants can't come off
        self::assertSame(['legs'], $avatar->update($user, ['slots' => ['legs' => null]]));
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs']);
        // shorts on again, shorts off → the swim pants come back on
        $avatar->update($user, ['slots' => ['legs' => $e['scutoons_shorts']]]);
        self::assertSame(['legs'], $avatar->update($user, ['slots' => ['legs' => null]]));
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs']);
        // shirt and sandals are no cover slots
        self::assertSame([], $avatar->update($user, ['slots' => ['torso' => null, 'feet' => null]]));
        self::assertSame([null, null], [$this->slots($user)['torso'], $this->slots($user)['feet']]);

        // setting off: naked allowed
        $accounts = $this->get(AccountService::class);
        $row = $this->db->one('SELECT * FROM users WHERE id = ?', [$user]) ?? [];
        $accounts->update($row, ['hide_sensitive' => false]);
        self::assertSame([], $avatar->update($user, ['slots' => ['legs' => null]]));
        self::assertNull($this->slots($user)['legs']);
        // on again: the pants come back
        $out = $accounts->update($row, ['hide_sensitive' => true]);
        self::assertSame($e['scutoons_swim_pants'], ((array) $out['avatar']['slots'])['legs']);
    }

    public function testTwinsGetEveryLegsItemThereIs(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $e = $this->entries($user);
        $this->get(CharacterUnlocks::class)->grant($user, 'toptoy', [], 'admin', null);
        $avatar = $this->get(AvatarService::class);
        // switching to the twins: the shorts stay on the first figure, the swim pants cover the second
        self::assertSame(['legs-right'], $avatar->update($user, ['body' => ['character' => 'toptoy', 'look' => 'swim']]));
        $slots = $this->slots($user);
        self::assertSame([$e['scutoons_shorts'], $e['scutoons_swim_pants'], $e['scutoons_tshirt'], $e['scutoons_sandals']], [$slots['legs-left'], $slots['legs-right'], $slots['torso-left'], $slots['feet-left']]);
        // back to a single figure: one legs slot, the rule keeps working
        $avatar->update($user, ['body' => ['character' => 'catpope', 'look' => 'swim']]);
        self::assertSame($e['scutoons_shorts'], $this->slots($user)['legs']);
    }

    public function testCoverRule(): void
    {
        $inst = [['id' => 'legs-left', 'slot' => 'legs'], ['id' => 'legs-right', 'slot' => 'legs'], ['id' => 'torso-left', 'slot' => 'torso']];
        [$worn, $restored] = AvatarService::cover(['legs-left' => 'a', 'legs-right' => null, 'torso-left' => null], $inst, [['id' => 'a', 'slot' => 'legs'], ['id' => 't', 'slot' => 'torso']]);
        self::assertSame([[], null, null], [$restored, $worn['legs-right'], $worn['torso-left']], 'one pair of pants only covers one figure, the torso is never refilled');
    }

    /**
     * MariaDB/MySQL only: two signups in flight at the same moment (two connections, both transactions open) must not
     * block each other. The basic dress used to DELETE a not-yet-existing avatar_slots row first – on InnoDB that takes
     * a gap lock, the other signup's INSERT waits for it, and with both doing so it is a deadlock (1213; 3 of 8 parallel
     * signups failed in tests/claim-race.php).
     */
    public function testTwoSignupsAtOnceDoNotBlockEachOther(): void
    {
        $this->registry();
        $second = \Scutoons\Kernel::container(\Scutoons\Infrastructure\Config\Settings::fixed(self::root(), $this->settings()));
        $second->set(\Scutoons\Infrastructure\Clock\Clock::class, $this->clock);
        $db2 = $second->get(\Scutoons\Infrastructure\Database\Database::class);
        $db2->exec('SET SESSION innodb_lock_wait_timeout = 1');
        $users = $this->get(\Scutoons\Domain\Accounts\Users::class);
        $this->db->pdo()->beginTransaction();
        $db2->pdo()->beginTransaction();
        try {
            $first = $users->create('first@example.com', 'en');
            $other = $second->get(\Scutoons\Domain\Accounts\Users::class)->create('second@example.com', 'en');
            self::assertNotSame($first['id'], $other['id']);
            self::assertSame(3, (int) $db2->value('SELECT COUNT(*) FROM avatar_slots WHERE user_id = ?', [$other['id']]), 'the basic dress is on');
        } finally {
            $db2->pdo()->rollBack();
            $this->db->pdo()->rollBack();
            $db2->close();
        }
    }
}
