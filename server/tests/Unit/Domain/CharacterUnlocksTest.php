<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Avatar\AvatarService;
use Scutoons\Domain\Characters\CharacterUnlocks;
use Scutoons\Tests\TestCase;

final class CharacterUnlocksTest extends TestCase
{
    private function registry(): void
    {
        $look = fn(string $c, string $l, bool $default, bool $image = true) => ['id' => $l, 'default' => $default] + ($image ? ['path' => "characters/$c/{$c}_apose-$l.webp"] : []);
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'apose' => ['looks' => [$look('catpope', 'swim', true), $look('catpope', 'naked', false)]]],
            ['id' => 'yellowmouse', 'apose' => ['looks' => [$look('yellowmouse', 'swim', true), $look('yellowmouse', 'naked', false, false)]]],
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
        self::assertSame(['catpope' => ['swim', 'naked']], $unlocks->defaults());
        self::assertSame([['character' => 'catpope', 'looks' => ['swim', 'naked'], 'source' => 'default']], $unlocks->toJson($this->userId()));
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
        self::assertFalse($unlocks->isUnlocked($user, 'yellowmouse', 'naked'));
        self::assertTrue($unlocks->grant($user, 'yellowmouse', ['naked'], 'story', null));
        self::assertSame(['swim', 'naked'], $unlocks->forUser($user)[1]['looks']);
        self::assertSame(['code', 'ABC123'], [$unlocks->forUser($user)[1]['source'], $unlocks->forUser($user)[1]['ref']]);

        // unlocked but no A-pose image → cannot be worn
        $avatar = $this->get(AvatarService::class);
        try {
            $avatar->update($user, ['body' => ['character' => 'yellowmouse', 'look' => 'naked']]);
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
        self::assertSame(['torso-left' => null, 'torso-right' => null], (array) $avatar->toJson($user)['slots']);
    }

    public function testSensitiveLooksFollowHideSensitive(): void
    {
        $json = json_encode(['characters' => [
            ['id' => 'catpope', 'apose' => ['looks' => [
                ['id' => 'swim', 'default' => true, 'path' => 'characters/catpope/catpope_apose-swim.webp'],
                ['id' => 'naked', 'sensitive' => true, 'path' => 'characters/catpope/catpope_apose-naked.webp'],
            ]]],
        ]]);
        $this->db->run("INSERT INTO entity_versions (entity_key, kind, toon, locale, json, author, created_at, status, message) VALUES ('characters', 'characters', NULL, NULL, ?, NULL, '2026-09-30 00:00:00', 'published', NULL)", [$json]);
        $unlocks = $this->get(CharacterUnlocks::class);
        $avatar = $this->get(AvatarService::class);
        $accounts = $this->get(AccountService::class);
        $user = $this->user();
        $id = (string) $user['id'];
        self::assertTrue($unlocks->isSensitive('catpope', 'naked'));
        self::assertFalse($unlocks->isSensitive('catpope', 'swim'));
        self::assertSame('swim', $unlocks->safeLook($id, 'catpope'));

        // on by default: a sensitive look can't be chosen
        self::assertTrue($avatar->hidesSensitive($id));
        self::assertTrue($accounts->me($user)['user']['hide_sensitive']);
        try {
            $avatar->update($id, ['body' => ['character' => 'catpope', 'look' => 'naked']]);
            self::fail('sensitive_hidden expected');
        } catch (ApiError $e) {
            self::assertSame(['sensitive_hidden', 422], [$e->errorCode, $e->status]);
        }

        // off: the naked look can be worn
        $out = $accounts->update($user, ['hide_sensitive' => false]);
        self::assertFalse($out['user']['hide_sensitive']);
        $avatar->update($id, ['body' => ['character' => 'catpope', 'look' => 'naked']]);
        self::assertSame(['character' => 'catpope', 'look' => 'naked'], $avatar->body($id));

        // on again: the body moves to the same character's first non-sensitive look (stored, and in the answer)
        $out = $accounts->update($user, ['hide_sensitive' => true]);
        self::assertTrue($out['user']['hide_sensitive']);
        self::assertSame(['character' => 'catpope', 'look' => 'swim'], $out['avatar']['body']);
        self::assertSame('swim', $this->db->value('SELECT look_id FROM avatar_body WHERE user_id = ?', [$id]));

        // a sensitive body stored before (e.g. before the setting existed) reads as the safe look while hidden
        $this->db->run("UPDATE avatar_body SET look_id = 'naked' WHERE user_id = ?", [$id]);
        self::assertSame(['character' => 'catpope', 'look' => 'swim'], $avatar->body($id));

        foreach ([[], ['hide_sensitive' => 'yes'], ['marketing_opt_in' => true, 'hide_sensitive' => 1]] as $bad) {
            try {
                $accounts->update($user, $bad);
                self::fail('invalid_body expected');
            } catch (ApiError $e) {
                self::assertSame('invalid_body', $e->errorCode);
            }
        }
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
