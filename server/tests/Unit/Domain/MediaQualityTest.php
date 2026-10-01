<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Tests\TestCase;

/** The member setting "Media quality" (users.media_quality, GET/PUT /me/; agents/DESIGN.md "Media quality") */
final class MediaQualityTest extends TestCase
{
    public function testAutoByDefaultStoredPerMemberAndValidated(): void
    {
        $accounts = $this->get(AccountService::class);
        $user = $this->user();
        $id = (string) $user['id'];

        self::assertSame('auto', $accounts->me($user)['user']['media_quality']);

        foreach (['low', 'medium', 'high', 'auto', 'high'] as $quality) {
            $out = $accounts->update($user, ['media_quality' => $quality]);
            self::assertSame($quality, $out['user']['media_quality']);
            self::assertSame($quality, $this->db->value('SELECT media_quality FROM users WHERE id = ?', [$id]));
        }
        // together with the other settings, which stay untouched otherwise
        $out = $accounts->update($user, ['media_quality' => 'low', 'marketing_opt_in' => true]);
        self::assertSame('low', $out['user']['media_quality']);
        self::assertTrue($out['user']['marketing_opt_in']);
        self::assertTrue($out['user']['hide_sensitive']);
        $out = $accounts->update($user, ['hide_sensitive' => false]);
        self::assertSame('low', $out['user']['media_quality']);

        foreach ([['media_quality' => 'ultra'], ['media_quality' => ''], ['media_quality' => 2], ['media_quality' => null], ['media_quality' => 'low', 'hide_sensitive' => 'yes']] as $bad) {
            try {
                $accounts->update($user, $bad);
                self::fail('invalid_body expected for ' . json_encode($bad));
            } catch (ApiError $e) {
                self::assertSame(['invalid_body', 400], [$e->errorCode, $e->status]);
            }
        }
        self::assertSame('low', $this->db->value('SELECT media_quality FROM users WHERE id = ?', [$id]));
    }

    public function testRowsBeforeTheMigrationReadAsAuto(): void
    {
        self::assertSame('auto', Users::mediaQuality([]));
        self::assertSame('auto', Users::mediaQuality(['media_quality' => null]));
        self::assertSame('auto', Users::mediaQuality(['media_quality' => 'bogus']));
        self::assertSame('medium', Users::mediaQuality(['media_quality' => 'medium']));
    }
}
