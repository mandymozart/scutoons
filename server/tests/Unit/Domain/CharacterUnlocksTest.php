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
    /**
     * A look of a persona; `apose` = the A-pose path ('' = an A-pose without an image), null = none
     * @param list<array<string, mixed>>|null $slots
     * @return array<string, mixed>
     */
    private static function look(string $id, bool $default, ?string $apose = null, ?array $slots = null): array
    {
        $look = ['id' => $id, 'default' => $default, 'images' => (object) []];
        if ($apose !== null) $look['apose'] = ($apose !== '' ? ['path' => $apose] : []) + ($slots !== null ? ['slots' => $slots] : []);
        return $look;
    }

    private function registry(): void
    {
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'personas' => [
                ['id' => 'regular', 'looks' => [self::look('default', true, 'characters/catpope/catpope_apose.webp')]],
                ['id' => 'biker', 'looks' => [self::look('default', true), self::look('helmet', false, 'characters/catpope-biker/catpope-biker_helmet_apose.webp')]],
            ]],
            ['id' => 'yellowmouse', 'personas' => [
                ['id' => 'regular', 'looks' => [self::look('default', true, 'characters/yellowmouse/yellowmouse_apose.webp'), self::look('winter', false, '')]],
            ]],
            ['id' => 'toptoy', 'personas' => [['id' => 'regular', 'looks' => [self::look('default', true)]]]],
        ]]);
        $this->storeCollection('characters', (array) json_decode((string) $json, true), 'published', '2026-09-30 00:00:00');
    }

    private function userId(): string
    {
        return (string) $this->user()['id'];
    }

    public function testDefaultsComeFromOneSetting(): void
    {
        $this->registry();
        $unlocks = $this->get(CharacterUnlocks::class);
        self::assertSame(['catpope'], $unlocks->defaults());
        self::assertSame([['character' => 'catpope', 'looks' => ['regular/default'], 'source' => 'default']], $unlocks->toJson($this->userId()));
        self::assertSame(['persona' => 'regular', 'look' => 'default'], $unlocks->defaultLook('catpope'));
        self::assertNull($unlocks->defaultLook('toptoy'), 'no look with an A-pose image');
    }

    public function testGrantUnlocksTheDefaultBodyAndFurtherLooks(): void
    {
        $this->registry();
        $unlocks = $this->get(CharacterUnlocks::class);
        $user = $this->userId();
        self::assertFalse($unlocks->isUnlocked($user, 'yellowmouse', 'regular', 'default'));
        self::assertTrue($unlocks->grant($user, 'yellowmouse', [], 'code', 'ABC123'));
        self::assertFalse($unlocks->grant($user, 'yellowmouse', [], 'code', 'ABC123'));
        self::assertTrue($unlocks->isUnlocked($user, 'yellowmouse', 'regular', 'default'));
        self::assertSame(['code', 'ABC123'], [$unlocks->forUser($user)[1]['source'], $unlocks->forUser($user)[1]['ref']]);
        // a further look of another persona, by `<persona>/<look>`
        self::assertFalse($unlocks->isUnlocked($user, 'catpope', 'biker', 'helmet'));
        self::assertTrue($unlocks->grant($user, 'catpope', ['biker/helmet'], 'story', null));
        self::assertSame(['regular/default', 'biker/helmet'], $unlocks->forUser($user)[0]['looks']);

        $avatar = $this->get(AvatarService::class);
        // a look without an A-pose image can't be granted (nor worn)
        try {
            $unlocks->grant($user, 'yellowmouse', ['regular/winter'], 'admin', null);
            self::fail('invalid_look expected');
        } catch (ApiError $e) {
            self::assertSame('invalid_look', $e->errorCode);
        }
        $avatar->update($user, ['body' => ['character' => 'catpope', 'persona' => 'biker', 'look' => 'helmet']]);
        self::assertSame(['character' => 'catpope', 'persona' => 'biker', 'look' => 'helmet'], $avatar->body($user));
        self::assertSame('biker', $this->db->value('SELECT persona_id FROM avatar_body WHERE user_id = ?', [$user]));
        // the same look id in another persona is another body
        try {
            $avatar->update($user, ['body' => ['character' => 'catpope', 'persona' => 'biker', 'look' => 'default']]);
            self::fail('locked expected');
        } catch (ApiError $e) {
            self::assertSame('locked', $e->errorCode);
        }
        try {
            $avatar->update($user, ['body' => ['character' => 'catpope', 'look' => 'default']]);
            self::fail('invalid_body expected');
        } catch (ApiError $e) {
            self::assertSame('invalid_body', $e->errorCode);
        }

        // revoking the look resets the worn body; the character stays
        self::assertTrue($unlocks->revoke($user, 'catpope', 'biker', 'helmet'));
        self::assertNull($avatar->body($user));
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM avatar_body'));
        self::assertTrue($unlocks->revoke($user, 'yellowmouse'));
        self::assertSame(['catpope'], array_column($unlocks->forUser($user), 'character'));
    }

    public function testSlotInstancesAndTheDefaultBody(): void
    {
        $slot = fn(string $id, string $type) => ['id' => $id, 'slot' => $type];
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'personas' => [['id' => 'regular', 'looks' => [self::look('default', true, 'characters/catpope/catpope_apose.webp')]]]],
            ['id' => 'toptoy', 'personas' => [['id' => 'regular', 'looks' => [
                self::look('default', true, 'characters/toptoy/toptoy_apose.webp', [$slot('torso-left', 'torso'), $slot('torso-right', 'torso'), ['id' => 'broken']]),
            ]]]],
        ]]);
        $this->storeCollection('characters', (array) json_decode((string) $json, true), 'published', '2026-09-30 00:00:00');
        $unlocks = $this->get(CharacterUnlocks::class);
        $avatar = $this->get(AvatarService::class);
        self::assertSame([['id' => 'torso-left', 'slot' => 'torso'], ['id' => 'torso-right', 'slot' => 'torso']], $unlocks->slotInstances('toptoy', 'regular', 'default'));
        self::assertSame([], $unlocks->slotInstances('catpope', 'regular', 'default'));
        // a body without slots: one instance per catalog slot type
        self::assertContains(['id' => 'torso', 'slot' => 'torso'], $avatar->instances(['character' => 'catpope', 'persona' => 'regular', 'look' => 'default']));
        self::assertSame(['character' => 'catpope', 'persona' => 'regular', 'look' => 'default'], $unlocks->defaultBody($this->userId()));
        $user = $this->userId();
        $unlocks->grant($user, 'toptoy', [], 'admin', null);
        $avatar->update($user, ['body' => ['character' => 'toptoy', 'persona' => 'regular', 'look' => 'default']]);
        // the default T-shirt moves onto the first torso instance (the registry's toptoy has no legs slots)
        $shirt = (string) $this->db->value("SELECT id FROM inventory WHERE user_id = ? AND item_id = 'scutoons_tshirt'", [$user]);
        self::assertSame(['torso-left' => $shirt, 'torso-right' => null], (array) $avatar->toJson($user)['slots']);
    }

    public function testTheHideSensitiveSettingLeavesTheAvatarAlone(): void
    {
        $this->registry();
        $avatar = $this->get(AvatarService::class);
        $accounts = $this->get(AccountService::class);
        $user = $this->user();
        $id = (string) $user['id'];
        // legs may be empty (no cover rule)
        $avatar->update($id, ['slots' => ['legs' => null]]);
        self::assertNull(((array) $avatar->toJson($id)['slots'])['legs'] ?? null);
        // the account setting stays (it governs sensitive slices): on by default, switchable, validated
        self::assertTrue($accounts->me($user)['user']['hide_sensitive']);
        $out = $accounts->update($user, ['hide_sensitive' => false]);
        self::assertFalse($out['user']['hide_sensitive']);
        self::assertTrue($accounts->update($user, ['hide_sensitive' => true])['user']['hide_sensitive']);
        self::assertNull(((array) $avatar->toJson($id)['slots'])['legs'] ?? null, 'turning it on puts nothing on');
        // "Show hints" (BACKLOG M.1): off by default, switchable, validated like the others
        self::assertFalse($accounts->me($user)['user']['show_hints']);
        self::assertTrue($accounts->update($user, ['show_hints' => true])['user']['show_hints']);
        self::assertFalse($accounts->update($user, ['show_hints' => false])['user']['show_hints']);
        foreach ([[], ['hide_sensitive' => 'yes'], ['marketing_opt_in' => true, 'hide_sensitive' => 1], ['show_hints' => 'on']] as $bad) {
            try {
                $accounts->update($user, $bad);
                self::fail('invalid_body expected');
            } catch (ApiError $e) {
                self::assertSame('invalid_body', $e->errorCode);
            }
        }
    }

    /** Migration 2026-10-13-apose-per-look: the character-level A-pose → the regular default look; member rows; the setting. */
    public function testTheAposePerLookMigration(): void
    {
        $slot = fn(string $id) => ['id' => $id, 'slot' => 'torso', 'x' => 372, 'y' => 340, 'size' => 330.5];
        $old = ['characters' => [
            ['id' => 'toptoy', 'apose' => ['looks' => [
                ['id' => 'swim', 'label' => ['de' => 'Badehose'], 'default' => true, 'path' => 'characters/toptoy/toptoy_apose-swim.webp', 'slots' => [$slot('torso-left') + ['label' => ['de' => 'links']], $slot('torso-right')]],
                ['id' => 'naked', 'sensitive' => true, 'path' => 'characters/toptoy/toptoy_apose-naked.webp'],
            ]], 'personas' => [
                ['id' => 'biker', 'looks' => [self::look('default', true)]],
                ['id' => 'regular', 'looks' => [self::look('cap', false), self::look('plain', true)]],
            ]],
            ['id' => 'boy', 'personas' => [['id' => 'regular', 'looks' => [self::look('default', true)]]]],
        ]];
        $this->storeCollection('characters', $old, 'published', '2026-09-30 00:00:00');
        $id = $this->userId();
        $other = $this->userId();
        $this->db->run("INSERT INTO avatar_body (user_id, character_id, look_id, updated_at) VALUES (?, 'toptoy', 'naked', '2026-10-01 00:00:00')", [$id]);
        $this->db->run("INSERT INTO member_character_looks (user_id, character_id, look_id, source, ref, unlocked_at) VALUES (?, 'toptoy', 'swim', 'code', 'ABC', '2026-10-02 00:00:00')", [$other]);
        $this->db->run('INSERT INTO settings (`key`, `value`, updated_at, updated_by) VALUES (?, ?, ?, ?)', ['AVATAR_DEFAULT_UNLOCKS', '{"catpope": ["swim"], "toptoy": []}', '2026-10-01 00:00:00', 'cli']);
        $runner = new MigrationRunner($this->db, self::root() . '/migrations');
        $file = self::root() . '/migrations/mysql/2026-10-13-apose-per-look.mysql.sql';
        $runner->runFile($file);
        $doc = fn() => json_decode((string) $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'character:toptoy'"), true);
        $after = $doc();
        $runner->runFile($file); // idempotent
        self::assertSame($after, $doc());
        self::assertArrayNotHasKey('apose', $after);
        self::assertSame(['path' => 'characters/toptoy/toptoy_apose-swim.webp', 'slots' => [
            ['id' => 'torso-left', 'slot' => 'torso', 'label' => ['de' => 'links']],
            ['id' => 'torso-right', 'slot' => 'torso'],
        ]], $after['personas'][1]['looks'][1]['apose'], 'the regular persona\'s default look');
        self::assertArrayNotHasKey('apose', $after['personas'][1]['looks'][0]);
        self::assertArrayNotHasKey('apose', $after['personas'][0]['looks'][0]);
        self::assertStringNotContainsString('apose', (string) $this->db->value("SELECT json FROM entity_versions WHERE entity_key = 'character:boy'"));
        // the worn body → the regular default look; a swim unlock → the character unlock
        self::assertSame(['toptoy', 'regular', 'plain'], array_values((array) $this->db->one('SELECT character_id, persona_id, look_id FROM avatar_body WHERE user_id = ?', [$id])));
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM member_character_looks'));
        self::assertSame(['toptoy', 'code', 'ABC'], array_values((array) $this->db->one('SELECT character_id, source, ref FROM member_characters WHERE user_id = ?', [$other])));
        self::assertSame(['catpope', 'toptoy'], json_decode((string) $this->db->value("SELECT `value` FROM settings WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'"), true));
        // the same in a JSON string (an override stored as text)
        $this->db->run("UPDATE settings SET `value` = ? WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'", [json_encode('{"catpope":["swim"]}')]);
        $runner->runFile($file);
        self::assertSame('["catpope"]', json_decode((string) $this->db->value("SELECT `value` FROM settings WHERE `key` = 'AVATAR_DEFAULT_UNLOCKS'"), true));
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
        foreach ([['nobody', [], 'admin', 'unknown_character'], ['yellowmouse', ['regular/glitter'], 'admin', 'invalid_look'], ['yellowmouse', ['winter'], 'admin', 'invalid_look'], ['yellowmouse', [], 'magic', 'invalid_source']] as [$c, $looks, $source, $error]) {
            try {
                $unlocks->grant($user, $c, $looks, $source, null);
                self::fail("$error expected");
            } catch (ApiError $e) {
                self::assertSame($error, $e->errorCode);
            }
        }
    }
}
