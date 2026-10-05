<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use InvalidArgumentException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Domain\Mail\MailRecipients;
use Scutoons\Domain\Mail\MailRenderer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Tests\TestCase;

/** The email module (Domain\Mail): registry, rendering de/en, sending through the log transport, the mail log, admin recipients */
final class MailTest extends TestCase
{
    /** Every registered template renders in every locale with its sample vars, every {placeholder} filled */
    public function testEveryTemplateRendersInEveryLocale(): void
    {
        $renderer = $this->get(MailRenderer::class);
        self::assertSame(['login_code', 'signup_without_code', 'ownership_claim_received', 'ownership_claim_admin', 'ownership_claim_rejected', 'ownership_transferred_from', 'ownership_transferred_to', 'gift_received', 'review_requested', 'review_published', 'review_returned', 'embassy_portion'], MailTemplates::ids());
        foreach (MailTemplates::ids() as $id) {
            $definition = MailTemplates::get($id);
            self::assertSame(array_keys($definition['vars']), array_keys($definition['sample']), "$id: a sample for every var");
            $subjects = [];
            foreach (MailTemplates::LOCALES as $locale) {
                $mail = $renderer->render($id, $locale, $definition['sample']);
                foreach ($mail as $part => $content) {
                    self::assertNotSame('', $content, "$id $locale $part");
                    self::assertDoesNotMatchRegularExpression('/\{[a-z_]+\}/', $content, "$id $locale $part: unfilled placeholder");
                }
                self::assertStringContainsString("<html lang=\"$locale\">", $mail['html']);
                self::assertStringEndsWith("\n\n– SCUTOONS\n", $mail['text']);
                $subjects[] = $mail['subject'];
            }
            self::assertCount(count(MailTemplates::LOCALES), array_unique($subjects), "$id: translated subjects");
        }
    }

    public function testLoginCodeTextsAndVariables(): void
    {
        $renderer = $this->get(MailRenderer::class);
        $vars = ['link' => 'https://scutoons.com/?login=T&x=<y>', 'otp' => '012345', 'minutes' => 30];
        $en = $renderer->render(MailTemplates::LOGIN_CODE, 'en-GB', $vars);
        self::assertSame('012345 is your SCUTOONS code', $en['subject'], 'the subject the tests and scripts grep for');
        self::assertSame("Confirm your email address to sign in to SCUTOONS.\n\nhttps://scutoons.com/?login=T&x=<y>\n\nOr enter this code:\n\n    012345\n\n"
            . "The link and the code work for 30 minutes.\nYou did not ask for this? Just ignore this email – nothing happens without your confirmation.\n\n– SCUTOONS\n", $en['text']);
        self::assertStringContainsString('href="https://scutoons.com/?login=T&amp;x=&lt;y&gt;"', $en['html'], 'HTML escaped');
        self::assertStringNotContainsString('inventory', $en['text'], 'no claim line without a code');

        $de = $renderer->render(MailTemplates::LOGIN_CODE, 'de', $vars + ['code' => 'Ab3', 'signup' => true]);
        self::assertSame('012345 ist dein SCUTOONS-Code', $de['subject']);
        self::assertStringContainsString("Damit wird dein SCUTOONS-Konto erstellt.\n\nNach der Anmeldung wird der Code Ab3 deinem Inventar hinzugefügt.", $de['text']);
        self::assertStringContainsString('<br>Du hast das nicht angefordert?', $de['html'], 'fine print joined by a line break');
        self::assertSame('en', MailTemplates::locale('fr'), 'unknown locale → en');
    }

    public function testSignupWithoutCodeHasNoLinkOrOneTimeCode(): void
    {
        $mail = $this->get(MailRenderer::class)->render(MailTemplates::SIGNUP_WITHOUT_CODE, 'en', ['url' => 'https://scutoons.com']);
        self::assertSame('Your SCUTOONS sign-in', $mail['subject']);
        self::assertStringContainsString('There is no account for it yet', $mail['text']);
        self::assertStringNotContainsString('cannot be used', $mail['text'], 'the code line only with a code');
        self::assertDoesNotMatchRegularExpression('/^\s{4}\d{6}\s*$/m', $mail['text']);
        $withCode = $this->get(MailRenderer::class)->render(MailTemplates::SIGNUP_WITHOUT_CODE, 'de', ['url' => 'https://scutoons.com', 'code' => 'xyz']);
        self::assertStringContainsString('Mit dem Code xyz kann kein Konto erstellt werden', $withCode['text']);
    }

    public function testUnknownTemplatesAndMissingVariablesAreBugs(): void
    {
        $renderer = $this->get(MailRenderer::class);
        foreach ([fn() => $renderer->render('nope', 'en', []), fn() => $renderer->render(MailTemplates::LOGIN_CODE, 'en', ['link' => 'x', 'otp' => '1']),
                     fn() => $renderer->render(MailTemplates::LOGIN_CODE, 'en', ['link' => 'x', 'otp' => ['1'], 'minutes' => 1])] as $call) {
            try {
                $call();
                self::fail('expected InvalidArgumentException');
            } catch (InvalidArgumentException) {
                self::addToAssertionCount(1);
            }
        }
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM mail_log'), 'nothing sent, nothing logged');
    }

    public function testSendWritesTheMailLogFileAndTheLogTable(): void
    {
        $id = $this->get(Mailer::class)->send(MailTemplates::LOGIN_CODE, 'reader@example.com', 'de-AT', ['link' => 'https://x.test/?login=a', 'otp' => '654321', 'minutes' => 30]);
        self::assertGreaterThan(0, $id);
        $file = (string) file_get_contents($this->dir . '/mail.log');
        self::assertMatchesRegularExpression('/^=== \S+ ===\r\nFrom: SCUTOONS <no-reply@scutoons\.com>\r\n/', $file, 'log transport format unchanged');
        self::assertStringContainsString("To: reader@example.com\r\nSubject: 654321 ist dein SCUTOONS-Code\r\n\r\n", $file);
        self::assertStringContainsString("\n    654321\n", $file);

        $row = $this->db->one('SELECT * FROM mail_log WHERE id = ?', [$id]);
        self::assertSame(['login_code', 'reader@example.com', 'de', 'sent', 'log', null, gmdate('Y-m-d H:i:s', self::NOW)],
            [$row['template'] ?? null, $row['recipient'] ?? null, $row['locale'] ?? null, $row['status'] ?? null, $row['transport'] ?? null, $row['error'] ?? null, $row['created_at'] ?? null]);
        self::assertStringNotContainsString('654321', (string) json_encode($row), 'never the one-time code in the table');
    }

    public function testAFailedSendIsLoggedWithTheError(): void
    {
        $this->container->set(\Scutoons\Infrastructure\Mail\Transport::class, new class(\Scutoons\Infrastructure\Config\Settings::fixed(self::root(), $this->settings())) extends \Scutoons\Infrastructure\Mail\Transport {
            public function deliver(string $to, string $subject, string $text, string $html): void
            {
                throw new \RuntimeException('SMTP 550 mailbox unavailable');
            }
        });
        try {
            $this->get(Mailer::class)->send(MailTemplates::SIGNUP_WITHOUT_CODE, 'gone@example.com', 'en', ['url' => 'https://scutoons.com']);
            self::fail('expected MailFailed');
        } catch (MailFailed $error) {
            self::assertSame('signup_without_code', $error->template);
            self::assertGreaterThan(0, $error->logId);
        }
        $row = $this->db->one('SELECT status, error FROM mail_log WHERE recipient = ?', ['gone@example.com']);
        self::assertSame(['failed', 'SMTP 550 mailbox unavailable'], [$row['status'] ?? null, $row['error'] ?? null]);
    }

    public function testTheLogListFiltersAndPrunesOldRows(): void
    {
        $log = $this->get(MailLog::class);
        $this->clock->advance(-200 * 86400);
        $log->record('login_code', 'old@example.com', 'en', 'sent', 'log', null);
        $this->clock->advance(200 * 86400);
        $log->record('login_code', 'a@example.com', 'en', 'sent', 'smtp', null);
        $log->record('signup_without_code', 'b@example.com', 'de', 'failed', 'smtp', 'timeout');
        self::assertNull($this->db->value('SELECT 1 FROM mail_log WHERE recipient = ?', ['old@example.com']), 'older than MAIL_LOG_DAYS (180) pruned');

        $list = fn(array $q) => $log->list(ListQuery::from($q, MailLog::SORTABLE, '-created_at', MailLog::FILTERS));
        self::assertSame(2, $list([])['pagination']['total']);
        self::assertSame(['b@example.com'], array_column($list(['status' => 'failed'])['items'], 'recipient'));
        self::assertSame(['a@example.com'], array_column($list(['template' => 'login_code'])['items'], 'recipient'));
        self::assertSame(['b@example.com'], array_column($list(['q' => 'B@EXA'])['items'], 'recipient'));
        self::assertSame(['a@example.com', 'b@example.com'], array_column($list(['sort' => 'recipient'])['items'], 'recipient'));
        foreach ([['status' => 'x'], ['template' => 'x'], ['from' => '1.1.2026']] as $q) {
            try {
                $list($q);
                self::fail('rejected: ' . json_encode($q));
            } catch (ApiError $error) {
                self::assertSame(400, $error->status);
            }
        }
    }

    public function testAdminRecipientsSettingAndSendToAdmins(): void
    {
        $recipients = $this->get(MailRecipients::class);
        self::assertSame(['items' => [], 'default' => [], 'stored' => false, 'updated_at' => null, 'updated_by' => null, 'max' => MailRecipients::MAX_ITEMS], $recipients->state());
        self::assertSame(['sent' => 0, 'failed' => 0], $this->get(Mailer::class)->sendToAdmins(MailTemplates::SIGNUP_WITHOUT_CODE, ['url' => 'https://scutoons.com']));

        self::assertTrue($recipients->update([' Boss@Example.com', 'ops@example.com', 'boss@example.com'], 'admin:boss@example.com'));
        self::assertSame(['boss@example.com', 'ops@example.com'], $recipients->admins());
        self::assertFalse($recipients->update(['boss@example.com', 'ops@example.com'], 'admin:x'), 'unchanged');
        self::assertSame('admin:boss@example.com', $recipients->state()['updated_by']);

        self::assertSame(['sent' => 2, 'failed' => 0], $this->get(Mailer::class)->sendToAdmins(MailTemplates::SIGNUP_WITHOUT_CODE, ['url' => 'https://scutoons.com']));
        self::assertSame(['de', 'de'], array_column($this->db->all("SELECT locale FROM mail_log WHERE recipient LIKE '%@example.com'"), 'locale'), 'MAIL_ADMIN_LOCALE');

        foreach (['nope', ['not-an-email'], array_map(fn($i) => "a$i@example.com", range(1, 11))] as $bad) {
            try {
                $recipients->update($bad, 'x');
                self::fail('rejected');
            } catch (ApiError $error) {
                self::assertSame([400, 'invalid_recipients'], [$error->status, $error->errorCode]);
            }
        }
        self::assertTrue($recipients->update(null, 'x'));
        self::assertFalse($recipients->state()['stored']);
    }

    public function testDefaultRecipientsComeFromTheConfig(): void
    {
        $container = \Scutoons\Kernel::container(\Scutoons\Infrastructure\Config\Settings::fixed(self::root(), ['MAIL_ADMIN_TO' => 'a@example.com, bad, B@example.com'] + $this->settings()));
        self::assertSame(['a@example.com', 'b@example.com'], $container->get(MailRecipients::class)->admins());
    }
}
