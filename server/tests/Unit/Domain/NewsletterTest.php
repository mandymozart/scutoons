<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Newsletter\Audiences;
use Scutoons\Domain\Newsletter\Campaigns;
use Scutoons\Domain\Newsletter\Contacts;
use Scutoons\Domain\Newsletter\NewsletterRenderer;
use Scutoons\Domain\Newsletter\NewsletterSender;
use Scutoons\Domain\Newsletter\Suppressions;
use Scutoons\Domain\Newsletter\UnsubscribeTokens;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\SmtpFailure;
use Scutoons\Infrastructure\Mail\Transport;
use Scutoons\Tests\TestCase;

/** Newsletters (Domain\Newsletter, agents/NEWSLETTER.md): contacts + import, audiences, start, batches, bounces, unsubscribe */
final class NewsletterTest extends TestCase
{
    protected function settings(): array
    {
        return ['NEWSLETTER_DELAY_MS' => '0', 'API_URL' => 'http://api.test'] + parent::settings();
    }

    /**
     * A transport that records every mail and fails the addresses in $failures with their SmtpFailure
     * @param array<string, SmtpFailure> $failures
     */
    private function fakeTransport(array $failures = []): FakeNewsletterTransport
    {
        $transport = new FakeNewsletterTransport(Settings::fixed(self::root(), $this->settings()), $failures);
        $this->container->set(Transport::class, $transport);
        return $transport;
    }

    /**
     * @param list<string> $audiences
     * @param array<string, mixed> $fields
     */
    private function campaign(array $audiences, array $fields = []): int
    {
        return (int) $this->get(Campaigns::class)->create($fields + [
            'title' => 'Chapter 3 is out',
            'audiences' => $audiences,
            'subject_de' => 'Kapitel 3 ist da, {name|du}',
            'subject_en' => 'Chapter 3 is out, {name|there}',
            'body_de' => "Hallo {name|du},\n\nKapitel 3 ist da.\n\n[Jetzt lesen](https://scutoons.com/catpope)",
            'body_en' => "Hi {name|there},\n\nchapter 3 is out.\nWith a new character.\n\n[Read now](https://scutoons.com/catpope)",
        ], 'admin:boss@example.com')['id'];
    }

    private static function errorCode(callable $work): ?string
    {
        try {
            $work();
        } catch (ApiError $error) {
            return $error->errorCode;
        }
        return null;
    }

    public function testTheMigrationCopiesTheOldQrFormsAddresses(): void
    {
        $this->code('membership', ['email' => ' Old@Example.com ']);
        $this->code('membership', ['email' => 'old@example.com']); // same person, second code
        $this->code('membership', ['email' => 'member@example.com']); // got an account meanwhile
        $this->user('member@example.com');
        $this->code('membership'); // no email
        $sql = (string) file_get_contents(self::root() . '/migrations/mysql/2026-10-07-newsletter.mysql.sql');
        $insert = substr($sql, (int) strpos($sql, 'INSERT IGNORE INTO `mail_contacts`'));
        $this->db->exec($insert);
        $this->db->exec($insert); // idempotent
        $rows = $this->db->all('SELECT email, source, locale, created_by FROM mail_contacts');
        self::assertSame([['email' => 'old@example.com', 'source' => 'qr-claim', 'locale' => null, 'created_by' => 'migration']], $rows);
    }

    public function testImportSortsEveryLine(): void
    {
        $contacts = $this->get(Contacts::class);
        $this->user('member@example.com');
        $this->get(Suppressions::class)->add('gone@example.com', 'unsubscribe');
        $contacts->import('known@example.com', 'event:Leipzig', null, null, false, 'admin:x');
        $text = "ana@example.com, Ana, en\nBEN@example.com;de\nmember@example.com\ngone@example.com\nknown@example.com\nana@example.com\nnot an address\n\n\"Cleo\"\tcleo@example.com";
        $preview = $contacts->import($text, 'event:Berlin 2026-09', '2026-09-12', 'de', true, 'admin:x');
        self::assertSame(['new', 'new', 'member', 'suppressed', 'exists', 'duplicate', 'invalid', 'new'], array_column($preview['rows'], 'status'));
        self::assertSame([1, 2, 3, 4, 5, 6, 7, 9], array_column($preview['rows'], 'line'));
        self::assertSame(['new' => 3, 'exists' => 1, 'member' => 1, 'suppressed' => 1, 'duplicate' => 1, 'invalid' => 1], $preview['counts']);
        self::assertSame(0, $preview['added'], 'a dry run adds nothing');
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM mail_contacts'));

        self::assertSame(3, $contacts->import($text, 'event:Berlin 2026-09', '2026-09-12', 'de', false, 'admin:x')['added']);
        $rows = $this->db->all("SELECT email, name, locale, consented_at FROM mail_contacts WHERE source = 'event:Berlin 2026-09' ORDER BY id");
        self::assertSame([
            ['email' => 'ana@example.com', 'name' => 'Ana', 'locale' => 'en', 'consented_at' => '2026-09-12'],
            ['email' => 'ben@example.com', 'name' => null, 'locale' => 'de', 'consented_at' => '2026-09-12'],
            ['email' => 'cleo@example.com', 'name' => 'Cleo', 'locale' => 'de', 'consented_at' => '2026-09-12'],
        ], $rows);

        self::assertSame('invalid_source', self::errorCode(fn() => $contacts->import('a@example.com', '', null, null, true, 'admin:x')));
    }

    public function testContactsLocalesCanBeSetForSeveralAtOnce(): void
    {
        $contacts = $this->get(Contacts::class);
        $contacts->import("a@example.com\nb@example.com\nc@example.com", 'qr-claim', null, null, false, 'admin:x');
        $ids = array_map('intval', array_column($this->db->all('SELECT id FROM mail_contacts ORDER BY id'), 'id'));
        self::assertSame(2, $contacts->updateMany(['ids' => [$ids[0], $ids[1]], 'locale' => 'de']));
        self::assertSame(['qr-claim' => 1], array_column(array_map(fn($s) => ['k' => $s['source'], 'v' => $s['missing_locale']], $contacts->sources()), 'v', 'k'));
        self::assertSame('en', $contacts->update($ids[2], ['locale' => 'en', 'name' => ' Cleo '])['locale']);
        self::assertSame(1, $contacts->delete([$ids[0]]));
    }

    public function testAudiencesEachAddressOnceMembersFirstNoSuppressed(): void
    {
        $users = $this->get(Users::class);
        $optedIn = $this->user('in@example.com');
        $users->setMarketingOptIn((string) $optedIn['id'], true);
        $this->db->run("UPDATE users SET locale = 'de', display_name = 'Ina' WHERE id = ?", [$optedIn['id']]);
        $this->user('out@example.com');
        $this->get(Contacts::class)->import("in@example.com\nqr@example.com, en\nnolang@example.com", 'qr-claim', null, null, false, 'admin:x');
        $this->get(Contacts::class)->import("event@example.com, de\nblocked@example.com, de", 'event:Leipzig', null, null, false, 'admin:x');
        $this->get(Suppressions::class)->add('blocked@example.com', 'manual');
        $audiences = $this->get(Audiences::class);

        self::assertSame(['in@example.com'], array_column($audiences->recipients([Audiences::MEMBERS_OPTED_IN]), 'email'));
        $all = $audiences->recipients(['contacts', 'members_opted_in']);
        self::assertSame(['in@example.com', 'qr@example.com', 'nolang@example.com', 'event@example.com'], array_column($all, 'email'), 'members first, then contacts, no suppressed');
        self::assertSame(['email' => 'in@example.com', 'locale' => 'de', 'name' => 'Ina', 'source' => 'member'], $all[0]);
        self::assertNull($all[2]['locale']);
        self::assertSame(['event@example.com'], array_column($audiences->recipients(['contacts:event:Leipzig']), 'email'));
        self::assertSame(['in@example.com', 'out@example.com'], array_column($audiences->recipients([Audiences::MEMBERS_ALL]), 'email'));

        $byId = array_column($audiences->all(), null, 'id');
        self::assertSame(['contacts', 'qr-claim', 2, 1], [$byId['contacts:qr-claim']['kind'], $byId['contacts:qr-claim']['source'], $byId['contacts:qr-claim']['count'], $byId['contacts:qr-claim']['missing_locale']]);
        self::assertSame(1, $byId['members_opted_in']['count']);
    }

    public function testStartChecksLanguagesAndTextsThenFixesTheRecipients(): void
    {
        $campaigns = $this->get(Campaigns::class);
        $this->get(Contacts::class)->import("a@example.com, de\nb@example.com", 'qr-claim', null, null, false, 'admin:x');
        $id = $this->campaign(['contacts'], ['body_en' => '']);
        try {
            $campaigns->start($id, 'admin:x');
            self::fail('missing_locale expected');
        } catch (ApiError $error) {
            self::assertSame(['missing_locale', 1, ['b@example.com']], [$error->errorCode, $error->extra['count'] ?? null, $error->extra['emails'] ?? null]);
        }
        $this->db->run("UPDATE mail_contacts SET locale = 'en' WHERE email = 'b@example.com'");
        try {
            $campaigns->start($id, 'admin:x');
            self::fail('missing_text expected');
        } catch (ApiError $error) {
            self::assertSame(['missing_text', 'en'], [$error->errorCode, $error->extra['locale'] ?? null]);
        }
        $campaigns->update($id, ['body_en' => 'Hi'], 'admin:x');
        $started = $campaigns->start($id, 'admin:x');
        self::assertSame(['sending', 2], [$started['status'], $started['progress']['queued']]);
        $this->get(Contacts::class)->import('late@example.com, de', 'qr-claim', null, null, false, 'admin:x');
        self::assertSame(2, $campaigns->get($id)['progress']['total'], 'a snapshot: later contacts are not added');

        self::assertSame('invalid_status', self::errorCode(fn() => $campaigns->update($id, ['subject_de' => 'x'], 'admin:x')));
    }

    public function testABatchSendsWithOneConnectionLogsAndFinishes(): void
    {
        $transport = $this->fakeTransport();
        $this->get(Contacts::class)->import("a@example.com, Ana, de\nb@example.com, en", 'qr-claim', null, null, false, 'admin:x');
        $id = $this->campaign(['contacts']);
        $this->get(Campaigns::class)->start($id, 'admin:x');

        $result = $this->get(NewsletterSender::class)->batch();
        self::assertSame(['sent', 2, 0, 0], [$result['status'], $result['sent'], $result['failed'], $result['queued']]);
        self::assertSame(1, $transport->batches, 'one SMTP batch (connection) for all mails');
        self::assertSame(['Kapitel 3 ist da, Ana', 'Chapter 3 is out, there'], array_column($transport->sent, 'subject'));
        self::assertStringContainsString("Hallo Ana,\n\nKapitel 3 ist da.\n\nJetzt lesen: https://scutoons.com/catpope\n\n– SCUTOONS", $transport->sent[0]['text']);
        self::assertStringContainsString('href="https://scutoons.com/catpope"', $transport->sent[0]['html'], 'the [text](url) line is a button');
        self::assertStringContainsString('With a new character.', $transport->sent[1]['text']);
        self::assertMatchesRegularExpression('#^List-Unsubscribe: <http://api\.test/api/newsletter/unsubscribe\?t=[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+>$#', $transport->sent[0]['headers'][0]);
        self::assertSame('List-Unsubscribe-Post: List-Unsubscribe=One-Click', $transport->sent[0]['headers'][1]);
        self::assertStringContainsString('Abmelden</a>', $transport->sent[0]['html']);
        self::assertSame(2, (int) $this->db->value("SELECT COUNT(*) FROM mail_log WHERE template = 'newsletter' AND status = 'sent'"));
        $campaign = $this->get(Campaigns::class)->get($id);
        self::assertSame(['done', 2], [$campaign['status'], $campaign['progress']['sent']]);
        self::assertSame('idle', $this->get(NewsletterSender::class)->batch()['status']);
    }

    public function testTheBatchSizeIsARuntimeSetting(): void
    {
        $transport = $this->fakeTransport();
        $this->get(RuntimeSettings::class)->update([RuntimeSettings::NEWSLETTER_BATCH => 2], 'admin:x');
        $this->get(Contacts::class)->import("a@example.com, de\nb@example.com, de\nc@example.com, de", 'qr-claim', null, null, false, 'admin:x');
        $this->get(Campaigns::class)->start($this->campaign(['contacts']), 'admin:x');
        self::assertSame([2, 1], [($r = $this->get(NewsletterSender::class)->batch())['sent'], $r['queued']]);
        self::assertSame(1, $this->get(NewsletterSender::class)->batch()['sent']);
        self::assertCount(3, $transport->sent);
        self::assertSame('invalid_setting', (function () {
            try {
                $this->get(RuntimeSettings::class)->update([RuntimeSettings::NEWSLETTER_BATCH => 26], 'admin:x');
            } catch (ApiError $error) {
                return $error->errorCode;
            }
            return null;
        })(), 'at most 25: a batch ends after NEWSLETTER_SECONDS anyway');
    }

    public function testTheHourlyLimitCountsEveryMailOfTheLastHour(): void
    {
        $transport = $this->fakeTransport();
        $this->get(RuntimeSettings::class)->update([RuntimeSettings::NEWSLETTER_PER_HOUR => 3], 'admin:x');
        $this->get(\Scutoons\Domain\Mail\MailLog::class)->record('login_code', 'x@example.com', 'en', 'sent', 'smtp', null);
        $this->get(\Scutoons\Domain\Mail\MailLog::class)->record('login_code', 'y@example.com', 'en', 'sent', 'smtp', null);
        $this->get(Contacts::class)->import("a@example.com, de\nb@example.com, de\nc@example.com, de", 'qr-claim', null, null, false, 'admin:x');
        $this->get(Campaigns::class)->start($this->campaign(['contacts']), 'admin:x');

        $first = $this->get(NewsletterSender::class)->batch();
        self::assertSame(['sent', 1, 2, 3], [$first['status'], $first['sent'], $first['queued'], $first['sent_last_hour']]);
        self::assertSame('limit', $this->get(NewsletterSender::class)->batch()['status']);
        $this->clock->advance(3601);
        self::assertSame(2, $this->get(NewsletterSender::class)->batch()['sent'], 'an hour later the rest goes');
        self::assertCount(3, $transport->sent);
    }

    public function testBouncesRetriesAndServerErrors(): void
    {
        $transport = $this->fakeTransport([
            'bad@example.com' => new SmtpFailure('SMTP: unexpected reply to RCPT TO:<bad@example.com>: 550 no such user', 'rcpt', 550),
            'busy@example.com' => new SmtpFailure('SMTP: unexpected reply to RCPT TO:<busy@example.com>: 451 try later', 'rcpt', 451),
        ]);
        $this->get(Contacts::class)->import("bad@example.com, de\nbusy@example.com, de\nok@example.com, de", 'qr-claim', null, null, false, 'admin:x');
        $id = $this->campaign(['contacts']);
        $this->get(Campaigns::class)->start($id, 'admin:x');
        $result = $this->get(NewsletterSender::class)->batch();
        self::assertSame([1, 1, 1, 1], [$result['sent'], $result['failed'], $result['retry'], $result['queued']]);
        self::assertTrue($this->get(Suppressions::class)->has('bad@example.com'), 'a 5xx at RCPT is a bounce: never again');
        $busy = $this->db->one("SELECT * FROM mail_campaign_recipients WHERE email = 'busy@example.com'");
        self::assertSame(['queued', 1], [$busy['status'] ?? null, (int) ($busy['attempts'] ?? 0)]);
        self::assertSame('idle', $this->get(NewsletterSender::class)->batch()['status'], 'not before NEWSLETTER_RETRY_MINUTES');

        $this->clock->advance(16 * 60);
        $transport->failures['busy@example.com'] = new SmtpFailure('SMTP: unexpected reply to AUTH (password hidden): 535 auth failed', 'auth', 535);
        $error = $this->get(NewsletterSender::class)->batch();
        self::assertSame(['error', 1], [$error['status'], $error['queued']]);
        self::assertStringContainsString('535', (string) $error['error']);
        self::assertSame(1, (int) $this->db->value("SELECT attempts FROM mail_campaign_recipients WHERE email = 'busy@example.com'"), 'a login problem counts no attempt');
        self::assertStringContainsString('535', (string) $this->get(Campaigns::class)->get($id)['last_error']);

        unset($transport->failures['busy@example.com']);
        self::assertSame('sent', $this->get(NewsletterSender::class)->batch()['status']);
        $campaign = $this->get(Campaigns::class)->get($id);
        self::assertSame(['done', null, 2, 1], [$campaign['status'], $campaign['last_error'], $campaign['progress']['sent'], $campaign['progress']['failed']]);
    }

    public function testPauseStopsTheBatchesAndASuppressionAfterTheStartSkips(): void
    {
        $transport = $this->fakeTransport();
        $this->get(Contacts::class)->import("a@example.com, de\nb@example.com, de", 'qr-claim', null, null, false, 'admin:x');
        $id = $this->campaign(['contacts']);
        $campaigns = $this->get(Campaigns::class);
        $campaigns->start($id, 'admin:x');
        $campaigns->pause($id, 'admin:x');
        self::assertSame('idle', $this->get(NewsletterSender::class)->batch()['status']);
        $campaigns->update($id, ['subject_de' => 'Neu'], 'admin:x');
        $this->get(Suppressions::class)->add('a@example.com', 'unsubscribe');
        $campaigns->resume($id, 'admin:x');
        $result = $this->get(NewsletterSender::class)->batch();
        self::assertSame([1, 1], [$result['sent'], $result['skipped']]);
        self::assertSame('Neu', $transport->sent[0]['subject'], 'a paused newsletter\'s fixed subject goes out');
    }

    public function testUnsubscribeTokensAndOptIn(): void
    {
        $tokens = $this->get(UnsubscribeTokens::class);
        $token = $tokens->token('Reader@Example.com', 'de');
        self::assertSame(['email' => 'reader@example.com', 'locale' => 'de'], $tokens->read($token));
        self::assertNull($tokens->read($token . 'x'));
        self::assertNull($tokens->read('abc.def'));
        self::assertNull($tokens->read(null));

        $users = $this->get(Users::class);
        $user = $this->user('reader@example.com');
        $users->setMarketingOptIn((string) $user['id'], true);
        $this->get(Suppressions::class)->add('reader@example.com', 'unsubscribe', 'link');
        self::assertSame(0, (int) $this->db->value('SELECT marketing_opt_in FROM users WHERE id = ?', [$user['id']]), 'an unsubscribe clears the opt-in');
        $users->setMarketingOptIn((string) $user['id'], true);
        self::assertFalse($this->get(Suppressions::class)->has('reader@example.com'), 'ticking it again lifts the unsubscribe');
        $this->get(Suppressions::class)->add('reader@example.com', 'bounce');
        $users->setMarketingOptIn((string) $user['id'], false);
        $users->setMarketingOptIn((string) $user['id'], true);
        self::assertTrue($this->get(Suppressions::class)->has('reader@example.com'), 'a bounce stays');
    }

    public function testRendererPlaceholdersAndEscaping(): void
    {
        $mail = $this->get(NewsletterRenderer::class)->render(['subject' => 'Hi {name}!', 'body' => "<b>{name|you}</b>\n\n[Go](https://x.test/?a=1&b=2)\n\n[not a button](javascript:alert(1))"], 'en', 'a@example.com', null);
        self::assertSame('Hi !', $mail['subject']);
        self::assertStringContainsString('&lt;b&gt;you&lt;/b&gt;', $mail['html']);
        self::assertStringContainsString('href="https://x.test/?a=1&amp;b=2"', $mail['html']);
        self::assertStringNotContainsString('href="javascript', $mail['html']);
        self::assertStringContainsString("No more news? Unsubscribe: http://api.test/api/newsletter/unsubscribe?t=", $mail['text']);
    }
}

/** The transport of NewsletterTest: records the mails, fails the addresses in $failures */
final class FakeNewsletterTransport extends Transport
{
    /** @var list<array{to: string, subject: string, text: string, html: string, headers: list<string>}> */
    public array $sent = [];
    public int $batches = 0;

    /** @param array<string, SmtpFailure> $failures */
    public function __construct(Settings $settings, public array $failures)
    {
        parent::__construct($settings);
    }

    public function beginBatch(): void
    {
        $this->batches++;
    }

    public function name(): string
    {
        return 'smtp';
    }

    public function deliverWithHeaders(string $to, string $subject, string $text, string $html, array $headers): void
    {
        if (isset($this->failures[$to])) throw $this->failures[$to];
        $this->sent[] = ['to' => $to, 'subject' => $subject, 'text' => $text, 'html' => $html, 'headers' => array_values($headers)];
    }
}
