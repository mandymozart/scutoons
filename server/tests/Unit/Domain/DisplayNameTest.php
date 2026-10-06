<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\AccountService;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Tests\TestCase;

/** The reader's name (users.display_name, GET/PUT /me/; agents/EXPRESSIONS.md §6 askText scope "reader", `{reader.name}`) */
final class DisplayNameTest extends TestCase
{
    public function testNoneByDefaultTrimmedPlainAndLimited(): void
    {
        $accounts = $this->get(AccountService::class);
        $user = $this->user();
        $id = (string) $user['id'];

        self::assertNull($accounts->me($user)['user']['display_name']);

        $out = $accounts->update($user, ['display_name' => "  Mia\n\t Sophie  "]);
        self::assertSame('Mia Sophie', $out['user']['display_name'], 'line breaks / tabs / runs of spaces become one space, trimmed');
        self::assertSame('Mia Sophie', $this->db->value('SELECT display_name FROM users WHERE id = ?', [$id]));
        // 32 characters (not bytes) fit
        $long = str_repeat('ä', Users::DISPLAY_NAME_MAX);
        self::assertSame($long, $accounts->update($user, ['display_name' => $long])['user']['display_name']);
        // markup stays text (the reader shows it as text, never as HTML)
        self::assertSame('<b>Mo</b>', $accounts->update($user, ['display_name' => '<b>Mo</b>'])['user']['display_name']);
        // together with another setting, which stays untouched otherwise
        $out = $accounts->update($user, ['display_name' => 'Ada', 'media_quality' => 'low']);
        self::assertSame(['Ada', 'low'], [$out['user']['display_name'], $out['user']['media_quality']]);
        // null / "" / blanks clear it
        foreach ([null, '', "  \n "] as $clear) {
            $accounts->update($user, ['display_name' => 'Ada']);
            self::assertNull($accounts->update($user, ['display_name' => $clear])['user']['display_name'], var_export($clear, true));
            self::assertNull($this->db->value('SELECT display_name FROM users WHERE id = ?', [$id]));
        }

        $accounts->update($user, ['display_name' => 'Ada']);
        foreach ([['display_name' => str_repeat('x', Users::DISPLAY_NAME_MAX + 1)], ['display_name' => 42], ['display_name' => ['Ada']], ['display_name' => 'Bo', 'hide_sensitive' => 'yes']] as $bad) {
            try {
                $accounts->update($user, $bad);
                self::fail('invalid_body expected for ' . json_encode($bad));
            } catch (ApiError $e) {
                self::assertSame(['invalid_body', 400], [$e->errorCode, $e->status]);
            }
        }
        self::assertSame('Ada', $this->db->value('SELECT display_name FROM users WHERE id = ?', [$id]), 'a refused update changes nothing');
    }
}
