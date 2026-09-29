<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Access\AccessTokens;
use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Tests\TestCase;

final class AccessTest extends TestCase
{
    public function testPermissionsMapIsData(): void
    {
        $permissions = $this->get(Permissions::class);
        self::assertSame(['claimable' => true, 'signup' => true, 'grants' => ['catalog']], $permissions->forType('membership'));
        self::assertSame(['claimable' => false, 'signup' => false, 'grants' => []], $permissions->forType('unknown-type'));
        $override = $this->dir . '/permissions.json';
        file_put_contents($override, json_encode(['types' => ['page' => ['claimable' => true, 'grants' => ['toon:catpope']]], 'default' => []]));
        $permissions->useFile($override);
        self::assertTrue($permissions->claimable('page'));
        self::assertFalse($permissions->claimable('membership'));
        $permissions->useFile(null);
        self::assertTrue($permissions->claimable('membership'));
    }

    public function testAccessTokensAreSignedAndExpire(): void
    {
        $tokens = $this->get(AccessTokens::class);
        $token = $tokens->issue(['catalog'], null, 'user:1');
        self::assertSame(self::NOW + AccessTokens::MAX_SECONDS, $tokens->verify($token)['exp'] ?? null);
        self::assertSame(self::NOW + 60, $tokens->verify($tokens->issue(['catalog'], self::NOW + 60, 'code:x'))['exp'] ?? null, 'exp = min(code expiry, 30 days)');
        self::assertNull($tokens->verify($tokens->issue(['catalog'], self::NOW - 1, 'code:x')));
        [$payload, $signature] = explode('.', $token);
        $forged = rtrim(strtr(base64_encode((string) json_encode(['v' => 1, 'perms' => ['admin'], 'exp' => self::NOW + 99, 'sub' => 'user:1'])), '+/', '-_'), '=');
        self::assertNull($tokens->verify("$forged.$signature"));
        self::assertNull($tokens->verify('garbage'));
    }

    public function testMeCombinesOwnedCodesAndThePromoSignupCode(): void
    {
        $promo = $this->code('promo');
        $user = $this->db->transaction(fn() => $this->get(\Scutoons\Domain\Accounts\Users::class)->create('promo@example.com', 'en', $promo));
        $me = $this->get(AccountService::class)->me($user);
        self::assertSame(['catalog'], $me['access']['permissions']);
        self::assertSame($promo, $me['access']['signup_code']);
        self::assertSame([], $me['memberships']);
        self::assertSame(['member'], $me['user']['roles']);
        $this->db->run('UPDATE qr SET revoked_at = ? WHERE code = ?', [$this->clock->local(), $promo]);
        $me = $this->get(AccountService::class)->me($user);
        self::assertFalse($me['access']['unlocked']);
        self::assertArrayNotHasKey('access_token', $me['access']);

        $vinyl = $this->code('membership', ['edition_id' => '3_catpope_vinyl']);
        $this->get(CodeService::class)->claim($user, $vinyl, null, null);
        $me = $this->get(AccountService::class)->me($user);
        self::assertSame([$vinyl], array_column($me['memberships'], 'code'));
        self::assertTrue($me['access']['unlocked']);
    }

    public function testRoles(): void
    {
        $roles = $this->get(Roles::class);
        $user = $this->user();
        self::assertSame(['member'], $roles->of((string) $user['id']));
        self::assertTrue($roles->grant((string) $user['id'], Roles::ADMIN, 'test'));
        self::assertFalse($roles->grant((string) $user['id'], Roles::ADMIN, 'test'));
        self::assertSame($user['id'], $roles->require($user, Roles::ADMIN)['id']);
        self::assertTrue($roles->revoke((string) $user['id'], Roles::ADMIN));
        try {
            $roles->require($user, Roles::ADMIN);
            self::fail('403 expected');
        } catch (ApiError $error) {
            self::assertSame([403, 'forbidden', ['role' => 'admin']], [$error->status, $error->errorCode, $error->extra]);
        }
        try {
            $roles->require(null, Roles::ADMIN);
            self::fail('401 expected');
        } catch (ApiError $error) {
            self::assertSame(401, $error->status);
        }
        $this->expectException(\InvalidArgumentException::class);
        $roles->grant((string) $user['id'], 'wizard');
    }
}
