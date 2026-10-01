<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Domain\Inventory\InventoryService;
use Scutoons\Tests\TestCase;

/** agents/ITEMS.md "Wardrobe": default wearables, swap, the cover rule while "Hide sensitive content" is on */
final class WardrobeTest extends TestCase
{
    private function registry(): void
    {
        $slot = fn(string $id, string $type) => ['id' => $id, 'slot' => $type, 'x' => 1, 'y' => 1, 'size' => 1];
        $single = array_map(fn(string $t) => $slot($t, $t), ['head', 'face', 'neck', 'torso', 'hands', 'legs', 'feet', 'back', 'accessory']);
        $twins = [$slot('torso-left', 'torso'), $slot('torso-right', 'torso'), $slot('legs-left', 'legs'), $slot('legs-right', 'legs')];
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

    public function testANewAccountWearsTheTshirtAndTheSwimPants(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $e = $this->entries($user);
        self::assertArrayHasKey('scutoons_shorts', $e, 'the shorts are a starter too (not worn)');
        $slots = $this->slots($user);
        self::assertSame($e['scutoons_tshirt'], $slots['torso']);
        self::assertSame($e['scutoons_swim_pants'], $slots['legs']);
    }

    public function testOlderAccountsGetTheDefaultWearablesOnce(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $this->db->run("DELETE FROM avatar_slots WHERE user_id = ?", [$user]);
        $this->db->run("DELETE FROM inventory WHERE user_id = ? AND item_id LIKE 'scutoons_%'", [$user]);
        $avatar = $this->get(AvatarService::class);
        self::assertSame(['scutoons_tshirt', 'scutoons_swim_pants', 'scutoons_shorts'], $avatar->ensureWardrobe($user));
        self::assertSame([], $avatar->ensureWardrobe($user), 'idempotent');
        $me = $this->get(AccountService::class)->me($this->db->one('SELECT * FROM users WHERE id = ?', [$user]) ?? []);
        self::assertCount(1, array_filter($me['inventory'], fn($x) => $x['item_id'] === 'scutoons_swim_pants'));
        self::assertNotNull($this->slots($user)['legs']);
    }

    public function testSwapKeepsTheOldItemAndTheCoverRuleRefillsLegs(): void
    {
        $this->registry();
        $user = (string) $this->user()['id'];
        $e = $this->entries($user);
        $avatar = $this->get(AvatarService::class);

        // shorts on → the swim pants come off but stay in the inventory
        self::assertSame([], $avatar->update($user, ['slots' => ['legs' => $e['scutoons_shorts']]]));
        self::assertSame($e['scutoons_shorts'], $this->slots($user)['legs']);
        self::assertSame(1, $this->get(InventoryService::class)->count($user, 'scutoons_swim_pants'));

        // hidden (default): taking the shorts off puts the swim pants back
        self::assertSame(['legs'], $avatar->update($user, ['slots' => ['legs' => null]]));
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs']);
        // … and the swim pants can't be taken off
        self::assertSame(['legs'], $avatar->update($user, ['slots' => ['legs' => null]]));
        // the shirt is not a cover slot
        self::assertSame([], $avatar->update($user, ['slots' => ['torso' => null]]));
        self::assertNull($this->slots($user)['torso']);

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
        // switching to the twins: the pants stay on the first figure, the shorts cover the second
        self::assertSame(['legs-right'], $avatar->update($user, ['body' => ['character' => 'toptoy', 'look' => 'swim']]));
        $slots = $this->slots($user);
        self::assertSame([$e['scutoons_swim_pants'], $e['scutoons_shorts'], $e['scutoons_tshirt']], [$slots['legs-left'], $slots['legs-right'], $slots['torso-left']]);
        // back to a single figure: one legs slot, the rule keeps working
        $avatar->update($user, ['body' => ['character' => 'catpope', 'look' => 'swim']]);
        self::assertSame($e['scutoons_swim_pants'], $this->slots($user)['legs']);
    }

    public function testCoverRule(): void
    {
        $inst = [['id' => 'legs-left', 'slot' => 'legs'], ['id' => 'legs-right', 'slot' => 'legs'], ['id' => 'torso-left', 'slot' => 'torso']];
        [$worn, $restored] = AvatarService::cover(['legs-left' => 'a', 'legs-right' => null, 'torso-left' => null], $inst, [['id' => 'a', 'slot' => 'legs'], ['id' => 't', 'slot' => 'torso']]);
        self::assertSame([[], null, null], [$restored, $worn['legs-right'], $worn['torso-left']], 'one pair of pants only covers one figure, the torso is never refilled');
    }
}
