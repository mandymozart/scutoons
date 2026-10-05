<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Domain\Embassy\EmbassyInbox;
use Scutoons\Domain\Embassy\EmbassyMail;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Mail\Mailbox;
use Scutoons\Kernel;
use Scutoons\Tests\Infrastructure\FakeMailbox;
use Scutoons\Tests\TestCase;

/** Embassy mail: sending from the shared mailbox (rules, footer, threading, log) and the inbox sync (matching, opt-out) */
final class EmbassyMailTest extends TestCase
{
    private FakeMailbox $mailbox;
    private string $transport = 'log';
    private string $imapHost = 'imap.test';

    protected function setUp(): void
    {
        parent::setUp();
        $this->useMailbox(new FakeMailbox());
    }

    protected function settings(): array
    {
        return ['MAIL_TRANSPORT' => $this->transport, 'EMBASSY_IMAP_HOST' => $this->imapHost, 'EMBASSY_IMAP_SENT' => 'Sent'] + parent::settings();
    }

    private function useMailbox(FakeMailbox $mailbox): void
    {
        $this->mailbox = $mailbox;
        $this->container->set(Mailbox::class, $mailbox);
    }

    /** Settings changed after setUp: a container with them (the database and the fake mailbox stay) */
    private function reload(): void
    {
        $clock = $this->clock;
        $this->db->close();
        $this->container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $this->container->set(Clock::class, $clock);
        $this->db = $this->container->get(Database::class);
        $this->useMailbox($this->mailbox);
    }

    /** @return array<string, mixed> */
    private function member(string $role, string $email = ''): array
    {
        $user = $this->user($email);
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return $user;
    }

    private function error(callable $fn): ApiError
    {
        try {
            $fn();
        } catch (ApiError $e) {
            return $e;
        }
        self::fail('Expected an ApiError');
    }

    /**
     * Venue in Vienna with Kathi (German), Max (do not contact) and an English person
     * @param array<string, mixed> $user
     * @param array<string, mixed> $fields
     * @return array<string, mixed> like Embassy::get()
     */
    private function venue(array $user, array $fields = []): array
    {
        return $this->get(Embassy::class)->create($fields + ['name' => 'Flex', 'type' => 'venue', 'city' => 'Wien', 'country' => 'AT', 'email' => 'Booking@Flex.at',
            'people' => [
                ['name' => 'Kathi', 'email' => 'kathi@flex.at', 'language' => 'de'],
                ['name' => 'Max', 'email' => 'max@flex.at', 'do_not_contact' => true],
                ['name' => 'Sam', 'email' => 'sam@flex.at', 'language' => 'en'],
            ]], $user);
    }

    /** @param array<string, mixed> $org */
    private static function personId(array $org, string $name): string
    {
        foreach ($org['people'] as $person) {
            if ($person['name'] === $name) return (string) $person['id'];
        }
        self::fail("no person $name");
    }

    private function mailLog(): string
    {
        return is_file($this->dir . '/mail.log') ? (string) file_get_contents($this->dir . '/mail.log') : '';
    }

    // ── Send ──────────────────────────────────────────────────────────────────────────────────────────────

    public function testSendRulesRefuseBeforeAnythingGoesOut(): void
    {
        $liaison = $this->member('liaison');
        $org = $this->venue($liaison);
        $id = $org['org']['id'];
        $other = $this->get(Embassy::class)->create(['name' => 'Other', 'people' => [['name' => 'Olga', 'email' => 'olga@other.at']]], $liaison);
        $mail = $this->get(EmbassyMail::class);
        $send = fn(array $data) => $this->error(fn() => $mail->send($id, $data + ['to' => 'kathi@flex.at', 'subject' => 'Hallo', 'text' => 'Text'], $liaison));

        foreach ([
            [['to' => 'stranger@example.com'], 400, 'invalid_recipient'],
            [['to' => 'olga@other.at'], 400, 'invalid_recipient'],
            [['to' => 'not an address'], 400, 'invalid_recipient'],
            [['to' => 'max@flex.at'], 409, 'do_not_contact'],
            [['person_id' => self::personId($other, 'Olga')], 400, 'invalid_person'],
            [['person_id' => self::personId($org, 'Max'), 'to' => 'booking@flex.at'], 409, 'do_not_contact'],
            [['subject' => "  \n "], 400, 'subject_required'],
            [['subject' => str_repeat('s', 301)], 400, 'subject_too_long'],
            [['text' => ' '], 400, 'text_required'],
            [['text' => str_repeat('t', 20001)], 400, 'text_too_long'],
            [['track' => 'circus'], 400, 'invalid_track'],
            [['reply_to_activity_id' => 'nope'], 400, 'invalid_reply_to'],
        ] as [$data, $status, $code]) {
            $e = $send($data);
            self::assertSame([$status, $code], [$e->status, $e->errorCode], (string) json_encode($data));
        }
        self::assertSame(403, $this->error(fn() => $mail->send($id, ['to' => 'kathi@flex.at', 'subject' => 'x', 'text' => 'y'], $this->member('')))->status);

        // the whole organisation opted out
        $this->get(Embassy::class)->update($id, ['do_not_contact' => true], $liaison);
        self::assertSame('do_not_contact', $send([])->errorCode);

        // a liaison can't mail on somebody else's lead (checked before sending)
        $theirs = $this->get(Embassy::class)->create(['name' => 'Theirs', 'email' => 'hi@theirs.at', 'leads' => [['track' => 'live']]], $this->member('liaison'))['org']['id'];
        self::assertSame('not_your_lead', $this->error(fn() => $mail->send($theirs, ['to' => 'hi@theirs.at', 'track' => 'live', 'subject' => 'x', 'text' => 'y'], $liaison))->errorCode);

        self::assertSame('', $this->mailLog());
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM embassy_activities'));
        self::assertSame(0, (int) $this->db->value('SELECT COUNT(*) FROM mail_log'));
        self::assertSame([], $this->mailbox->appended);
    }

    public function testSendLogsTheMailMovesTheLeadAndKeepsACopy(): void
    {
        $liaison = $this->member('liaison', 'lia@example.com');
        $org = $this->venue($liaison);
        $id = $org['org']['id'];
        $result = $this->get(EmbassyMail::class)->send($id, ['to' => 'Kathi@Flex.at', 'track' => 'live', 'subject' => "Top Toy\nlive",
            'text' => "Hallo Kathi,\r\nwir spielen im März in Wien.\n.Punkt am Zeilenanfang"], $liaison);

        $activity = $result['activity'];
        self::assertSame(['mail_out', 'live', self::personId($org, 'Kathi'), 'Top Toy live', 'hello@scutoons.com', 'Kathi@Flex.at', 'lia@example.com'],
            [$activity['kind'], $activity['track'], $activity['person_id'], $activity['subject'], $activity['mail_from'], $activity['mail_to'], $activity['by_email']]);
        self::assertMatchesRegularExpression('/^[0-9a-f-]{36}@scutoons\.com$/', (string) $activity['message_id']);
        self::assertSame("Hallo Kathi,\nwir spielen im März in Wien.\n.Punkt am Zeilenanfang\n\n" . EmbassyMail::FOOTERS['de'], $activity['note']);
        // the cadence of a sent mail: contacted, call in 6 days (Sunday → Monday), and the liaison owns the lead
        self::assertSame(['contacted', '2026-09-28', (string) $liaison['id']], [$result['lead']['stage'] ?? null, $result['lead']['next_at'] ?? null, $result['lead']['owner_id'] ?? null]);

        $log = $this->mailLog();
        self::assertStringContainsString('From: =?UTF-8?B?' . base64_encode('Top Toy · SCUTOONS') . '?= <hello@scutoons.com>', $log);
        self::assertStringContainsString("Reply-To: <hello@scutoons.com>\r\n", $log);
        self::assertStringContainsString('Message-ID: <' . $activity['message_id'] . ">\r\n", $log);
        self::assertStringContainsString("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: quoted-printable\r\nTo: Kathi@Flex.at\r\nSubject: Top Toy live\r\n", $log);
        self::assertStringContainsString('Einfach mit „Stopp“ antworten.', $log);
        self::assertStringNotContainsString('In-Reply-To', $log);
        $row = $this->db->one('SELECT template, recipient, locale, status, transport FROM mail_log');
        self::assertSame(['embassy', 'Kathi@Flex.at', 'de', 'sent', 'log'], array_values($row ?? []));

        // the copy in the shared mailbox's Sent folder: the encoded message as sent
        self::assertCount(1, $this->mailbox->appended);
        self::assertSame('Sent', $this->mailbox->appended[0]['folder']);
        self::assertStringContainsString("wir spielen im M=C3=A4rz in Wien.\r\n.Punkt", $this->mailbox->appended[0]['message']);
        self::assertSame(0, $this->mailbox->closed - 1);

        // shown on the organisation's timeline
        self::assertSame('Top Toy live', $this->get(Embassy::class)->get($id)['activities'][0]['subject']);
    }

    public function testFooterOnceAndInTheRecipientsLanguage(): void
    {
        $amb = $this->member('ambassador');
        $mail = $this->get(EmbassyMail::class);
        $org = $this->venue($amb);
        $id = $org['org']['id'];
        $send = fn(array $data) => $mail->send($id, $data + ['subject' => 'Hi'], $amb)['activity']['note'];

        // already there (any language, any case) → not again
        $text = "Hi Sam,\n\nsee you!\n\n—\nRATHER not hear from us? Just reply stop.";
        self::assertSame($text, $send(['to' => 'sam@flex.at', 'text' => $text]));
        self::assertSame(1, substr_count($send(['to' => 'sam@flex.at', 'text' => "Hi\n\n" . EmbassyMail::FOOTERS['de']]), 'Keine weiteren'));
        // the person's language; the organisation's address: German for an Austrian venue; asked explicitly: that one
        self::assertStringEndsWith(EmbassyMail::FOOTERS['en'], $send(['to' => 'sam@flex.at', 'text' => 'Hi']));
        self::assertStringEndsWith(EmbassyMail::FOOTERS['de'], $send(['to' => 'booking@flex.at', 'text' => 'Hallo']));
        self::assertStringEndsWith(EmbassyMail::FOOTERS['en'], $send(['to' => 'booking@flex.at', 'text' => 'Hello', 'language' => 'en']));
        $dutch = $this->get(Embassy::class)->create(['name' => 'Paradiso', 'country' => 'NL', 'email' => 'info@paradiso.nl'], $amb)['org']['id'];
        self::assertStringEndsWith(EmbassyMail::FOOTERS['en'], $mail->send($dutch, ['to' => 'info@paradiso.nl', 'subject' => 'Hi', 'text' => 'Hello'], $amb)['activity']['note']);
    }

    public function testReplyIsThreaded(): void
    {
        $amb = $this->member('ambassador');
        $mail = $this->get(EmbassyMail::class);
        $org = $this->venue($amb);
        $id = $org['org']['id'];
        $first = $mail->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Hi', 'text' => 'One'], $amb)['activity'];
        $second = $mail->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Re: Hi', 'text' => 'Two', 'reply_to_activity_id' => $first['id']], $amb)['activity'];
        $log = $this->mailLog();
        self::assertStringContainsString("In-Reply-To: <{$first['message_id']}>\r\nReferences: <{$first['message_id']}>\r\n", $log);
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(*) FROM embassy_activities WHERE in_reply_to = ?', [$first['message_id']]));
        self::assertNotSame($first['message_id'], $second['message_id']);

        // a note is no mail; another organisation's mail is not this thread
        $note = $this->get(Embassy::class)->log($id, ['kind' => 'note', 'note' => 'x'], $amb)['activity']['id'];
        self::assertSame('invalid_reply_to', $this->error(fn() => $mail->send($id, ['to' => 'kathi@flex.at', 'subject' => 'x', 'text' => 'y', 'reply_to_activity_id' => $note], $amb))->errorCode);
        $other = $this->get(Embassy::class)->create(['name' => 'Other', 'email' => 'o@other.at'], $amb)['org']['id'];
        self::assertSame('invalid_reply_to', $this->error(fn() => $mail->send($other, ['to' => 'o@other.at', 'subject' => 'x', 'text' => 'y', 'reply_to_activity_id' => $first['id']], $amb))->errorCode);
    }

    public function testTransportFailureLogsNothingOnTheOrganisation(): void
    {
        $amb = $this->member('ambassador');
        $id = $this->venue($amb)['org']['id'];
        $this->transport = 'carrier-pigeon';
        $this->reload();
        $e = $this->error(fn() => $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Hi', 'text' => 'Text'], $amb));
        self::assertSame([502, 'mail_failed'], [$e->status, $e->errorCode]);
        self::assertStringContainsString('Unknown MAIL_TRANSPORT', $e->getMessage());
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM embassy_activities WHERE kind = 'mail_out'"));
        self::assertSame(['embassy', 'failed'], array_values($this->db->one('SELECT template, status FROM mail_log') ?? []));
        self::assertSame([], $this->mailbox->appended);
    }

    public function testACopyThatFailsDoesNotFailTheSend(): void
    {
        $amb = $this->member('ambassador');
        $id = $this->venue($amb)['org']['id'];
        $this->mailbox->failOn = 'append';
        $result = $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'subject' => 'Hi', 'text' => 'Text'], $amb);
        self::assertSame('mail_out', $result['activity']['kind']);
        self::assertNull($result['lead']); // no track: no lead
        self::assertFalse($this->mailbox->open);

        // no IMAP host: no copy at all
        $this->imapHost = '';
        $this->reload();
        $this->mailbox->failOn = null;
        $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'subject' => 'Hi', 'text' => 'Text'], $amb);
        self::assertSame([], $this->mailbox->appended);
    }

    public function testSendWorksBeforeTheMigration(): void
    {
        if (!$this->db->isSqlite()) self::markTestSkipped('drops columns (the MySQL test database is only rebuilt for dropped tables)');
        $amb = $this->member('ambassador');
        $id = $this->venue($amb)['org']['id'];
        $this->db->exec('DROP INDEX embassy_activities_message');
        foreach (['subject', 'mail_from', 'mail_to', 'message_id', 'in_reply_to'] as $column) $this->db->exec("ALTER TABLE embassy_activities DROP COLUMN $column");
        $this->db->exec('DROP TABLE embassy_mail_state');
        $result = $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Hi', 'text' => 'Text'], $amb);
        self::assertSame(['mail_out', null, null, 'contacted'], [$result['activity']['kind'], $result['activity']['subject'], $result['activity']['message_id'], $result['lead']['stage'] ?? null]);
        self::assertStringContainsString('Subject: Hi', $this->mailLog());
        $e = $this->error(fn() => $this->get(EmbassyInbox::class)->run());
        self::assertSame([503, 'not_available'], [$e->status, $e->errorCode]);
    }

    // ── Inbox ─────────────────────────────────────────────────────────────────────────────────────────────

    /** @param array<string, string> $headers */
    private static function raw(array $headers, string $body): string
    {
        $lines = [];
        foreach ($headers + ['Date' => 'Mon, 21 Sep 2026 12:00:00 +0200', 'Content-Type' => 'text/plain; charset=utf-8'] as $name => $value) $lines[] = "$name: $value";
        return implode("\r\n", $lines) . "\r\n\r\n" . str_replace("\n", "\r\n", $body);
    }

    public function testInboxMatchesRepliesAndSkipsTheRest(): void
    {
        $amb = $this->member('ambassador');
        $org = $this->venue($amb);
        $id = $org['org']['id'];
        $sent = $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Top Toy live', 'text' => 'Hallo'], $amb)['activity'];
        $ours = $sent['message_id'];
        $this->mailbox->uidValidity = 7;
        $this->mailbox->mails = [
            10 => self::raw(['From' => 'Kathi <Kathi@Flex.at>', 'Subject' => 'Re: Top Toy live', 'Message-ID' => '<r1@flex.at>', 'In-Reply-To' => "<$ours>"],
                "Klingt gut, ruf mich an!\n\nAm Mo., 21. Sep. 2026 um 10:00 Uhr schrieb Top Toy · SCUTOONS <hello@scutoons.com>:\n> Hallo\n> —"),
            11 => self::raw(['From' => 'someone@unknown.example', 'Subject' => 'Offer', 'Message-ID' => '<spam@unknown>'], 'Buy now'),
            12 => self::raw(['From' => 'booking@flex.at', 'Subject' => 'Anfrage', 'Message-ID' => '<r2@flex.at>', 'Date' => 'Sun, 20 Sep 2026 22:30:00 -0100'], "Bitte schickt uns das EPK.\n-----Original Message-----\nFrom: x"),
            13 => self::raw(['From' => 'kathi@flex.at', 'Subject' => 'Re: Top Toy live', 'Message-ID' => '<r1@flex.at>', 'In-Reply-To' => "<$ours>"], 'the same mail again'),
            14 => self::raw(['From' => 'kathi@flex.at', 'Subject' => 'Abwesend', 'Message-ID' => '<ooo@flex.at>', 'Auto-Submitted' => 'auto-replied', 'In-Reply-To' => "<$ours>"], 'Bin im Urlaub'),
            15 => self::raw(['From' => 'sam@flex.at', 'Subject' => 'Re: Top Toy live', 'Message-ID' => '<r3@flex.at>', 'References' => "<older@x> <$ours>"],
                "I'm taking over from Kathi.\n\nOn Mon, Sep 21, 2026 at 10:00 AM Top Toy <hello@scutoons.com>\nwrote:\n> Hallo"),
            16 => self::raw(['From' => 'hello@scutoons.com', 'Subject' => 'our own', 'Message-ID' => '<own@scutoons.com>'], 'x'),
            17 => self::raw(['From' => 'MAILER-DAEMON@mx.flex.at', 'Subject' => 'Undelivered', 'Message-ID' => '<bounce@mx>', 'References' => "<$ours>"], 'failed'),
        ];
        $result = $this->get(EmbassyInbox::class)->run();
        self::assertSame(['configured' => true, 'synced_at' => '2026-09-21T14:13:20+00:00', 'matched' => 3, 'skipped' => 5, 'new_activities' => 3, 'error' => null], $result);
        self::assertSame([['after' => 0, 'since' => '2026-08-22']], $this->mailbox->searches); // first run: the last 30 days

        $in = $this->db->all("SELECT * FROM embassy_activities WHERE kind = 'mail_in' ORDER BY at");
        self::assertSame([
            ['r2@flex.at', null, 'booking@flex.at', 'Bitte schickt uns das EPK.', '2026-09-20 23:30:00'],
            ['r1@flex.at', self::personId($org, 'Kathi'), 'kathi@flex.at', 'Klingt gut, ruf mich an!', '2026-09-21 10:00:00'],
            ['r3@flex.at', self::personId($org, 'Sam'), 'sam@flex.at', "I'm taking over from Kathi.", '2026-09-21 10:00:00'],
        ], array_map(fn(array $a) => [$a['message_id'], $a['person_id'], $a['mail_from'], $a['note'], $a['at']], $in));
        self::assertSame(['inbox', null, 'Re: Top Toy live', $ours], [$in[1]['by_email'], $in[1]['by_user_id'], $in[1]['subject'], $in[1]['in_reply_to']]);
        // every reply is on the live lead (thread, or the organisation's latest mail_out): in conversation, sequence stopped
        self::assertSame(1, (int) $this->db->value('SELECT COUNT(DISTINCT lead_id) FROM embassy_activities WHERE lead_id IS NOT NULL'));
        $lead = $this->get(Embassy::class)->get($id)['leads'][0];
        self::assertSame(['in_conversation', '2026-10-01', (string) $amb['id']], [$lead['stage'], $lead['next_at'], $lead['owner_id']]);
        // nothing about the unknown sender, the bounce or our own mail is stored
        self::assertSame(0, (int) $this->db->value("SELECT COUNT(*) FROM embassy_activities WHERE note LIKE '%Buy now%' OR message_id IN ('spam@unknown', 'bounce@mx', 'own@scutoons.com', 'ooo@flex.at')"));

        $state = $this->db->one('SELECT * FROM embassy_mail_state');
        self::assertSame(['INBOX', 7, 17, null], [$state['id'] ?? null, (int) ($state['uid_validity'] ?? 0), (int) ($state['last_uid'] ?? 0), $state['error'] ?? null]);

        // the next run only asks for newer mails
        $this->mailbox->mails[18] = self::raw(['From' => 'kathi@flex.at', 'Subject' => 'Noch was', 'Message-ID' => '<r4@flex.at>'], 'Noch was.');
        self::assertSame([1, 0], [$this->get(EmbassyInbox::class)->run()['matched'] ?? null, $this->get(EmbassyInbox::class)->run()['matched'] ?? null]);
        self::assertSame([17, 18], array_column(array_slice($this->mailbox->searches, 1), 'after'));

        // a new UIDVALIDITY: read again from the start – already logged mails are not logged twice
        $this->mailbox->uidValidity = 8;
        $again = $this->get(EmbassyInbox::class)->run();
        self::assertSame([0, 9], [$again['matched'] ?? null, $again['skipped'] ?? null]);
        self::assertSame(0, $this->mailbox->searches[3]['after']);
        self::assertSame(4, (int) $this->db->value("SELECT COUNT(*) FROM embassy_activities WHERE kind = 'mail_in'"));
    }

    public function testStopReplySetsDoNotContactForever(): void
    {
        $amb = $this->member('ambassador');
        $org = $this->venue($amb);
        $id = $org['org']['id'];
        $ours = $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'track' => 'live', 'subject' => 'Hi', 'text' => 'Hallo'], $amb)['activity']['message_id'];
        $this->mailbox->mails = [
            1 => self::raw(['From' => 'kathi@flex.at', 'Subject' => 'Re: Hi', 'Message-ID' => '<s1@flex.at>', 'In-Reply-To' => "<$ours>"],
                "\n  STOPP!\n\nAm 21.09.2026 schrieb hello@scutoons.com:\n> Hallo"),
        ];
        self::assertSame(1, $this->get(EmbassyInbox::class)->run()['matched'] ?? null);
        $detail = $this->get(Embassy::class)->get($id);
        $kathi = array_values(array_filter($detail['people'], fn(array $p) => $p['name'] === 'Kathi'))[0];
        self::assertTrue($kathi['do_not_contact']);
        self::assertFalse($detail['org']['do_not_contact']);
        self::assertSame(['in_conversation', null], [$detail['leads'][0]['stage'], $detail['leads'][0]['next_at']]); // no follow-up
        $reply = array_values(array_filter($detail['activities'], fn(array $a) => $a['kind'] === 'mail_in'))[0];
        self::assertStringStartsWith("STOPP!\n\n[Opt-out:", (string) $reply['note']);
        $e = $this->error(fn() => $this->get(EmbassyMail::class)->send($id, ['to' => 'kathi@flex.at', 'subject' => 'Hi', 'text' => 'again'], $amb));
        self::assertSame('do_not_contact', $e->errorCode);

        // from the organisation's address: the whole organisation, lead closed
        $this->mailbox->mails[2] = self::raw(['From' => 'booking@flex.at', 'Subject' => 'Abmelden', 'Message-ID' => '<s2@flex.at>'], 'Unsubscribe.');
        $this->get(EmbassyInbox::class)->run();
        $detail = $this->get(Embassy::class)->get($id);
        self::assertSame([true, 'no', null], [$detail['org']['do_not_contact'], $detail['leads'][0]['stage'], $detail['leads'][0]['next_at']]);
    }

    public function testReplyStrippingAndStopWords(): void
    {
        self::assertSame("Gern!\n\nLG Anna", EmbassyInbox::reply("Gern!\n\nLG Anna\n\nOn Mon, 21 Sep 2026, Top Toy wrote:\n> Hallo\n> Welt"));
        self::assertSame('Yes', EmbassyInbox::reply("Yes\n> quoted\n>> deeper"));
        self::assertSame('Ja', EmbassyInbox::reply("Ja\n\nAm Montag, 21. September 2026 um 10:00 schrieb Top Toy <hello@scutoons.com>:\nHallo"));
        self::assertSame("Am Montag passt es.\nBis dann", EmbassyInbox::reply("Am Montag passt es.\nBis dann"));
        self::assertSame('Ok', EmbassyInbox::reply("Ok\n\n-----Original Message-----\nFrom: Top Toy"));
        self::assertSame('Passt', EmbassyInbox::reply("Passt\n\nVon: Top Toy <hello@scutoons.com>\nGesendet: Montag, 21. September 2026 10:00\nAn: Kathi\nBetreff: Hi"));
        self::assertSame("From: the label side\nwe like it", EmbassyInbox::reply("From: the label side\nwe like it"));
        self::assertSame('Gut', EmbassyInbox::reply("Gut\n________________________________\nFrom: x"));

        foreach (['stop', 'STOPP!', ' Stop. ', "\n\nunsubscribe\nthanks", 'Abmelden', '„Stopp“'] as $text) self::assertTrue(EmbassyInbox::isStop($text), $text);
        foreach (['stop it', 'Please stop', 'Bitte abmelden', '', "Hello\nstop"] as $text) self::assertFalse(EmbassyInbox::isStop($text), $text);
    }

    public function testSyncThrottleForceStatusAndErrors(): void
    {
        $liaison = $this->member('liaison');
        $amb = $this->member('ambassador');
        $inbox = $this->get(EmbassyInbox::class);
        self::assertSame(['configured' => true, 'synced_at' => null, 'error' => null], $inbox->status());

        $first = $inbox->sync($liaison, false);
        self::assertSame([false, '2026-09-21T14:13:20+00:00', null], [$first['throttled'], $first['synced_at'], $first['error']]);
        $this->clock->advance(60);
        self::assertTrue($inbox->sync($liaison, false)['throttled']);
        self::assertTrue($inbox->sync($liaison, true)['throttled']); // only ambassadors force
        self::assertFalse($inbox->sync($amb, true)['throttled']);
        self::assertCount(2, $this->mailbox->searches);
        $this->clock->advance(121);
        self::assertFalse($inbox->sync($liaison, false)['throttled']);

        // the mailbox fails: the error is stored and shown, the next run tries again
        $this->clock->advance(121);
        $this->mailbox->failOn = 'open';
        $failed = $inbox->sync($liaison, false);
        self::assertStringContainsString('IMAP connect', (string) $failed['error']);
        self::assertSame($failed['error'], $inbox->status()['error']);
        self::assertSame(403, $this->error(fn() => $inbox->sync($this->member(''), false))->status);

        // not configured
        $this->imapHost = '';
        $this->reload();
        self::assertSame(['configured' => false], $this->get(EmbassyInbox::class)->sync($liaison, false));
        self::assertSame(['configured' => false, 'synced_at' => null, 'error' => null], $this->get(EmbassyInbox::class)->status());
    }
}
