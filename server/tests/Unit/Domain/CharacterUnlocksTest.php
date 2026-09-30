<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

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
