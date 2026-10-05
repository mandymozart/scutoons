<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Scutoons\Tests\TestCase;

final class CharacterUnlocksTest extends TestCase
{
    private function registry(): void
    {
        $look = fn(string $c, string $l, bool $default, bool $image = true) => ['id' => $l, 'default' => $default] + ($image ? ['path' => "characters/$c/{$c}_apose-$l.webp"] : []);
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'apose' => ['looks' => [$look('catpope', 'swim', true), $look('catpope', 'winter', false)]]],
            ['id' => 'yellowmouse', 'apose' => ['looks' => [$look('yellowmouse', 'swim', true), $look('yellowmouse', 'winter', false, false)]]],
            ['id' => 'toptoy'],
        ]]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, '2026-09-30 00:00:00', 'published', NULL)", [$json]);
    }

    private function userId(): string
    {
        return (string) $this->user()['id'];
    }

    public function testDefaultsComeFromOneSetting(): void
    {
        $unlocks = $this->get(CharacterUnlocks::class);
        self::assertSame(['catpope' => ['swim']], $unlocks->defaults());
        self::assertSame([['character' => 'catpope', 'looks' => ['swim'], 'source' => 'default']], $unlocks->toJson($this->userId()));
    }

    public function testGrantUnlocksTheDefaultLookAndFurtherLooks(): void
    {
        $this->registry();
        $unlocks = $this->get(CharacterUnlocks::class);
        $user = $this->userId();
        self::assertFalse($unlocks->isUnlocked($user, 'yellowmouse', 'swim'));
        self::assertTrue($unlocks->grant($user, 'yellowmouse', [], 'code', 'ABC123'));
        self::assertFalse($unlocks->grant($user, 'yellowmouse', [], 'code', 'ABC123'));
        self::assertTrue($unlocks->isUnlocked($user, 'yellowmouse', 'swim'));
        self::assertFalse($unlocks->isUnlocked($user, 'yellowmouse', 'winter'));
        self::assertTrue($unlocks->grant($user, 'yellowmouse', ['winter'], 'story', null));
        self::assertSame(['swim', 'winter'], $unlocks->forUser($user)[1]['looks']);
        self::assertSame(['code', 'ABC123'], [$unlocks->forUser($user)[1]['source'], $unlocks->forUser($user)[1]['ref']]);

        // unlocked but no A-pose image → cannot be worn
        $avatar = $this->get(AvatarService::class);
        try {
            $avatar->update($user, ['body' => ['character' => 'yellowmouse', 'look' => 'winter']]);
            self::fail('no_apose expected');
        } catch (ApiError $e) {
            self::assertSame('no_apose', $e->errorCode);
        }
        $avatar->update($user, ['body' => ['character' => 'yellowmouse', 'look' => 'swim']]);
        self::assertSame(['character' => 'yellowmouse', 'look' => 'swim'], $avatar->body($user));

        // revoking resets the worn body
        self::assertTrue($unlocks->revoke($user, 'yellowmouse'));
        self::assertNull($avatar->body($user));
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM avatar_body'));
    }

    public function testSlotInstancesAndTheDefaultBody(): void
    {
        $slot = fn(string $id, string $type) => ['id' => $id, 'slot' => $type, 'x' => 1, 'y' => 1, 'size' => 1];
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'apose' => ['looks' => [['id' => 'swim', 'default' => true, 'path' => 'characters/catpope/catpope_apose-swim.webp']]]],
            ['id' => 'toptoy', 'apose' => ['looks' => [['id' => 'swim', 'default' => true, 'path' => 'characters/toptoy/toptoy_apose-swim.webp', 'slots' => [$slot('torso-left', 'torso'), $slot('torso-right', 'torso'), ['id' => 'broken']]]]]],
        ]]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, '2026-09-30 00:00:00', 'published', NULL)", [$json]);
        $unlocks = $this->get(CharacterUnlocks::class);
        $avatar = $this->get(AvatarService::class);
        self::assertSame([['id' => 'torso-left', 'slot' => 'torso'], ['id' => 'torso-right', 'slot' => 'torso']], $unlocks->slotInstances('toptoy', 'swim'));
        self::assertSame([], $unlocks->slotInstances('catpope', 'swim'));
        // a body without slots: one instance per catalog slot type
        self::assertContains(['id' => 'torso', 'slot' => 'torso'], $avatar->instances(['character' => 'catpope', 'look' => 'swim']));
        self::assertSame(['character' => 'catpope', 'look' => 'swim'], $unlocks->defaultBody($this->userId()));
        $user = $this->userId();
        $unlocks->grant($user, 'toptoy', [], 'admin', null);
        $avatar->update($user, ['body' => ['character' => 'toptoy', 'look' => 'swim']]);
        // the default T-shirt moves onto the first torso instance (the registry's toptoy has no legs slots)
        $shirt = (string) $this->db->value("SELECT id FROM inventory WHERE user_id = ? AND item_id = 'scutoons_tshirt'", [$user]);
        self::assertSame(['torso-left' => $shirt, 'torso-right' => null], (array) $avatar->toJson($user)['slots']);
    }

    /** The naked A-pose look was removed (owner 2026-10-05): leftovers of older registry data and rows are ignored. */
    public function testTheRetiredNakedLookIsIgnored(): void
    {
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'apose' => ['looks' => [
                ['id' => 'swim', 'default' => true, 'path' => 'characters/catpope/catpope_apose-swim.webp'],
                ['id' => 'naked', 'sensitive' => true, 'path' => 'characters/catpope/catpope_apose-naked.webp'],
                ['id' => 'beach', 'sensitive' => true, 'path' => 'characters/catpope/catpope_apose-beach.webp'],
            ]]],
        ]]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, '2026-09-30 00:00:00', 'published', NULL)", [$json]);
        $unlocks = $this->get(CharacterUnlocks::class);
        $avatar = $this->get(AvatarService::class);
        $accounts = $this->get(AccountService::class);
        $user = $this->user();
        $id = (string) $user['id'];
        self::assertTrue($unlocks->isRetired('catpope', 'naked'));
        self::assertTrue($unlocks->isRetired('catpope', 'beach'), 'a look still marked sensitive');
        self::assertFalse($unlocks->isRetired('catpope', 'swim'));
        self::assertFalse($unlocks->hasApose('catpope', 'naked'));

        // can't be granted or worn; an old unlock row is left out
        try {
            $unlocks->grant($id, 'catpope', ['naked'], 'admin', null);
            self::fail('invalid_look expected');
        } catch (ApiError $e) {
            self::assertSame('invalid_look', $e->errorCode);
        }
        $this->db->run("INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, 'catpope', 'naked', 'admin', NULL, '2026-10-01 00:00:00')", [$id]);
        self::assertSame([['character' => 'catpope', 'looks' => ['swim'], 'source' => 'default']], $unlocks->toJson($id));
        try {
            $avatar->update($id, ['body' => ['character' => 'catpope', 'look' => 'naked']]);
            self::fail('locked expected');
        } catch (ApiError $e) {
            self::assertSame('locked', $e->errorCode);
        }

        // a naked body stored before reads as the character's default look; legs may be empty (no cover rule)
        $this->db->run("INSERT INTO avatar_body (user_id, character_id, look_id, updated_at) VALUES (?, 'catpope', 'naked', '2026-10-01 00:00:00')", [$id]);
        self::assertSame(['character' => 'catpope', 'look' => 'swim'], $avatar->body($id));
        $avatar->update($id, ['slots' => ['legs' => null]]);
        self::assertNull(((array) $avatar->toJson($id)['slots'])['legs'] ?? null);

        // the account setting stays (it governs sensitive slices): on by default, switchable, validated
        self::assertTrue($accounts->me($user)['user']['hide_sensitive']);
        $out = $accounts->update($user, ['hide_sensitive' => false]);
        self::assertFalse($out['user']['hide_sensitive']);
        self::assertSame(['character' => 'catpope', 'look' => 'swim'], $out['avatar']['body']);
        self::assertTrue($accounts->update($user, ['hide_sensitive' => true])['user']['hide_sensitive']);
        self::assertNull(((array) $avatar->toJson($id)['slots'])['legs'] ?? null, 'turning it on puts nothing on');
        foreach ([[], ['hide_sensitive' => 'yes'], ['marketing_opt_in' => true, 'hide_sensitive' => 1]] as $bad) {
            try {
                $accounts->update($user, $bad);
                self::fail('invalid_body expected');
            } catch (ApiError $e) {
                self::assertSame('invalid_body', $e->errorCode);
            }
        }
    }

    public function testTheNoNakedMigration(): void
    {
        $id = $this->userId();
        $other = $this->userId();
        $this->db->run("INSERT INTO avatar_body (user_id, character_id, look_id, updated_at) VALUES (?, 'yellowmouse', 'naked', '2026-10-01 00:00:00')", [$id]);
        $this->db->run("INSERT INTO member_characters (user_id, character_id, source, ref, unlocked_at) VALUES (?, 'catpope', 'admin', NULL, '2026-10-01 00:00:00')", [$id]);
        $this->db->run("INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, 'catpope', 'naked', 'admin', NULL, '2026-10-01 00:00:00')", [$id]);
        $this->db->run("INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, 'yellowmouse', 'naked', 'code', 'ABC', '2026-10-02 00:00:00')", [$id]);
        $this->db->run("INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, 'yellowmouse', 'winter', 'story', NULL, '2026-10-02 00:00:00')", [$other]);
        $this->db->run('INSERT INTO settings (`key`, `value`, updated_at, updated_by) VALUES (?, ?, ?, ?)', ['AVATAR_DEFAULT_UNLOCKS', '{"catpope": ["swim", "naked"], "toptoy": ["naked"]}', '2026-10-01 00:00:00', 'cli']);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-05-no-naked.mysql.sql');
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-05-no-naked.mysql.sql'); // idempotent
        self::assertSame('swim', $this->db->value('SELECT look_id FROM avatar_body WHERE user_id = ?', [$id]));
        self::assertSame(['winter'], array_column($this->db->all('SELECT look_id FROM member_character_looks ORDER BY look_id'), 'look_id'));
        // the character a naked unlock brought along stays unlocked (with its default look)
        self::assertSame([['catpope', 'admin', null], ['yellowmouse', 'code', 'ABC']], array_map(
            fn(array $r) => [$r['character_id'], $r['source'], $r['ref']],
            $this->db->all('SELECT character_id, source, ref FROM member_characters WHERE user_id = ? ORDER BY character_id', [$id])
        ));
        self::assertSame(['catpope' => ['swim'], 'toptoy' => []], json_decode((string) $this->db->value("SELECT `value` FROM settings WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'"), true));
        // the same in a JSON string (an override stored as text)
        $this->db->run("UPDATE settings SET `value` = ? WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'", [json_encode('{"catpope":["swim","naked"]}')]);
        $runner->runFile(self::root() . '/migrations/mysql/2026-10-05-no-naked.mysql.sql');
        self::assertSame('{"catpope":["swim"]}', json_decode((string) $this->db->value("SELECT `value` FROM settings WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'"), true));
    }

    public function testValidation(): void
    {
        $this->registry();
        $unlocks = $this->get(CharacterUnlocks::class);
        $user = $this->userId();
        foreach ([['nobody', [], 'admin', 'unknown_character'], ['yellowmouse', ['glitter'], 'admin', 'invalid_look'], ['yellowmouse', [], 'magic', 'invalid_source']] as [$c, $looks, $source, $error]) {
            try {
                $unlocks->grant($user, $c, $looks, $source, null);
                self::fail("$error expected");
            } catch (ApiError $e) {
                self::assertSame($error, $e->errorCode);
            }
        }
    }
}
