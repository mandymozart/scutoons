<?php
declare(strict_types=1);

namespace Scutoons\Tests\Http;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Kernel;
use Scutoons\Support\Ids;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Embassy pace (owner 2026-10-05): onboarding + check-in, the weekly portion (fill order, cap, one item per
 * organisation, do-not-contact, done, today's share), the team overview, the cron URL with the reminder mails.
 * The test clock starts on Monday 2026-09-21 14:13 UTC (the API's time zone is UTC here).
 */
final class EmbassyPaceApiTest extends TestCase
{
    private const DAY = 86400;
    private string $cronKey = 'cron-secret-key';

    protected function settings(): array
    {
        return ['EMBASSY_CRON_KEY' => $this->cronKey, 'EMBASSY_URL' => 'http://embassy.test'] + parent::settings();
    }

    /** @param array<string, string> $headers */
    private function request(string $method, string $uri, array $headers = [], mixed $body = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        foreach ($headers as $name => $value) $request = $request->withHeader($name, $value);
        if ($body !== null) {
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
            $request = $request->withHeader('Content-Type', 'application/json');
        }
        $container = Kernel::container(Settings::fixed(self::root(), $this->settings()));
        $container->set(Clock::class, $this->clock);
        return Kernel::app($container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    /** @return array{0: array<string, string>, 1: string} auth headers + user id */
    private function auth(string $role, string $email = ''): array
    {
        $user = $this->user($email);
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return [['Authorization' => 'Bearer ' . $this->get(Sessions::class)->create((string) $user['id'])], (string) $user['id']];
    }

    /**
     * An organisation with leads (owned by whoever creates them)
     * @param array<string, string> $auth
     * @param list<array<string, mixed>> $leads
     * @param array<string, mixed> $fields
     */
    private function org(array $auth, string $name, array $leads = [['track' => 'live']], array $fields = []): string
    {
        $r = $this->request('POST', '/api/embassy/orgs', $auth, ['name' => $name, 'city' => 'Graz', 'leads' => $leads] + $fields);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        return (string) self::json($r)['org']['id'];
    }

    /**
     * @param array<string, string> $auth
     * @param array<string, mixed> $body
     */
    private function log(array $auth, string $orgId, array $body): void
    {
        $r = $this->request('POST', "/api/embassy/orgs/$orgId/activities", $auth, $body);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
    }

    /**
     * @param array<string, string> $auth
     * @return array<string, mixed>
     */
    private function portion(array $auth, string $owner = 'me'): array
    {
        $r = $this->request('GET', "/api/embassy/portion?owner=$owner", $auth);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        return self::json($r);
    }

    /**
     * @param array<string, mixed> $portion
     * @return list<string>
     */
    private static function names(array $portion): array
    {
        $names = [];
        foreach ((array) $portion['items'] as $item) $names[] = (string) $item['org']['name'];
        return $names;
    }

    /** @return array<string, mixed> */
    private function cron(): array
    {
        $r = $this->request('GET', '/api/embassy/cron?key=' . $this->cronKey);
        self::assertSame(200, $r->getStatusCode(), (string) $r->getBody());
        return self::json($r);
    }

    private function mailLog(): string
    {
        return is_file($this->dir . '/mail.log') ? (string) file_get_contents($this->dir . '/mail.log') : '';
    }

    public function testOnboardingAndValidation(): void
    {
        [$liaison] = $this->auth('liaison');
        self::assertNull(self::json($this->request('GET', '/api/embassy/me', $liaison))['pace']);
        self::assertSame(401, $this->request('PUT', '/api/embassy/pace', [], ['per_week' => 10])->getStatusCode());
        self::assertSame(403, $this->request('PUT', '/api/embassy/pace', $this->auth('')[0], ['per_week' => 10])->getStatusCode());

        foreach ([['per_week' => 0], ['per_week' => 101], ['per_week' => 'ten'], ['per_week' => 2.5]] as $body) {
            $r = $this->request('PUT', '/api/embassy/pace', $liaison, $body);
            self::assertSame([400, 'invalid_pace'], [$r->getStatusCode(), self::json($r)['error']], (string) json_encode($body));
        }
        $r = $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10, 'reminder' => 'hourly']);
        self::assertSame([400, 'invalid_reminder'], [$r->getStatusCode(), self::json($r)['error']]);
        self::assertNull(self::json($this->request('GET', '/api/embassy/me', $liaison))['pace'], 'nothing stored by a refused call');

        $r = $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 15, 'reminder' => 'daily']);
        self::assertSame(200, $r->getStatusCode());
        self::assertSame(['per_week' => 15, 'reminder' => 'daily', 'onboarded_at' => '2026-09-21T14:13:20+00:00', 'asked_at' => '2026-09-21T14:13:20+00:00', 'checkin_due' => false],
            self::json($r)['pace']);
        $pace = self::json($this->request('PUT', '/api/embassy/pace', $liaison, ['reminder' => 'off']))['pace'];
        self::assertSame([15, 'off'], [$pace['per_week'], $pace['reminder']], 'what is left out stays');
        self::assertSame(15, self::json($this->request('GET', '/api/embassy/me', $liaison))['pace']['per_week']);

        [$other] = $this->auth('liaison');
        self::assertSame(['per_week' => 10, 'reminder' => 'weekly'], array_intersect_key(
            self::json($this->request('PUT', '/api/embassy/pace', $other, []))['pace'], ['per_week' => 0, 'reminder' => 0]), 'the presets');
    }

    public function testCheckinMathAndTheFourWeekQuestion(): void
    {
        [$liaison] = $this->auth('liaison');
        $r = $this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'fine']);
        self::assertSame([409, 'not_onboarded'], [$r->getStatusCode(), self::json($r)['error']]);
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 3]);

        $due = fn() => self::json($this->request('GET', '/api/embassy/me', $liaison))['pace']['checkin_due'];
        $this->clock->advance(27 * self::DAY);
        self::assertFalse($due());
        $this->clock->advance(self::DAY);
        self::assertTrue($due(), '28 days after the onboarding');

        $r = $this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'maybe']);
        self::assertSame([400, 'invalid_answer'], [$r->getStatusCode(), self::json($r)['error']]);
        $pace = self::json($this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'less']))['pace'];
        self::assertSame([1, false], [$pace['per_week'], $pace['checkin_due']], '3 − 5 → the floor 1, asked now');
        self::assertSame(6, self::json($this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'more']))['pace']['per_week']);
        self::assertSame(6, self::json($this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'fine']))['pace']['per_week']);
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 98]);
        self::assertSame(100, self::json($this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'more']))['pace']['per_week'], 'the cap');
        $this->clock->advance(28 * self::DAY - 1);
        self::assertFalse($due(), 'counted from the last answer');
        self::assertSame(4, (int) $this->db->value('SELECT COUNT(*) FROM embassy_checkins'));
    }

    public function testPortionFillOrderCapOneItemPerOrgAndDoNotContact(): void
    {
        [$liaison, $liaisonId] = $this->auth('liaison');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 4]);
        $this->org($liaison, 'Fresh', [['track' => 'live']]);
        $this->org($liaison, 'Fresh A', [['track' => 'live']], ['tier' => 'A']);
        $dueOld = $this->org($liaison, 'Due old', [['track' => 'live', 'next_at' => '2026-09-18']]);
        $this->org($liaison, 'Due today', [['track' => 'live', 'next_at' => '2026-09-21']]);
        $twoLeads = $this->org($liaison, 'Two leads', [['track' => 'live', 'next_at' => '2026-09-19'], ['track' => 'vinyl']]);
        $this->org($liaison, 'Opted out', [['track' => 'live', 'next_at' => '2026-09-10']], ['do_not_contact' => true]);
        $quiet = $this->org($liaison, 'Quiet', [['track' => 'live', 'stage' => 'in_conversation']]);
        // they answered 100 days ago, nothing since
        $leadId = (string) $this->db->value('SELECT id FROM embassy_leads WHERE org_id = ?', [$quiet]);
        $this->db->run("INSERT INTO embassy_activities (id, org_id, lead_id, kind, note, by_email, at) VALUES (?, ?, ?, 'mail_in', '', 'inbox', ?)",
            [Ids::uuid7(), $quiet, $leadId, $this->clock->utc(-100 * self::DAY)]);
        [$other] = $this->auth('liaison');
        $this->org($other, 'Somebody else’s', [['track' => 'live', 'next_at' => '2026-09-01']]);

        $p = $this->portion($liaison);
        self::assertSame(['2026-09-21', ['start' => '2026-09-21', 'end' => '2026-09-27', 'pace' => 4, 'done' => 0, 'left' => 4], 8],
            [$p['today'], $p['week'], $p['owned']]);
        self::assertSame(['Due old', 'Two leads', 'Due today', 'Quiet'], self::names($p), 'due (oldest first), then quiet; one item per organisation; never do-not-contact');
        self::assertSame(['due', 'due', 'due', 'quiet'], array_column($p['items'], 'group'));
        self::assertSame(['due' => 0, 'quiet' => 0, 'first' => 2], $p['more'], 'Fresh A + Fresh did not fit');
        self::assertSame(['kind' => 'due', 'date' => '2026-09-18'], $p['items'][0]['hint']);
        self::assertSame(['kind' => 'answered_then_quiet', 'date' => '2026-06-13', 'days' => 100], $p['items'][3]['hint']);
        self::assertSame(['lead', 'org', 'people', 'last_activity'], array_slice(array_keys($p['items'][0]), 2), 'the Today item shape');
        self::assertSame(1, $p['today_count'], 'Monday: 4 left over 5 working days → 1');

        // a voicemail (kept due today) and a mail: both count as done – two organisations
        $this->log($liaison, $dueOld, ['kind' => 'call', 'track' => 'live', 'outcome' => 'voicemail', 'next_at' => '2026-09-21']);
        $this->log($liaison, $twoLeads, ['kind' => 'mail_out', 'track' => 'live', 'next_at' => '2026-09-21']);
        $this->log($liaison, $twoLeads, ['kind' => 'note', 'note' => 'a note is no contact']);
        $p = $this->portion($liaison);
        self::assertSame(['done' => 2, 'left' => 2], array_intersect_key($p['week'], ['done' => 0, 'left' => 0]));
        self::assertSame(['Due old', 'Due today'], self::names($p));
        self::assertSame(['kind' => 'voicemail', 'days' => 0], $p['items'][0]['hint']);
        self::assertSame(['due' => 1, 'quiet' => 1, 'first' => 2], $p['more']);
        self::assertSame([2, 0], [$p['today_done'], $p['today_count']], 'today’s share (1) is done already');

        // somebody else's portion: ambassadors only
        $r = $this->request('GET', "/api/embassy/portion?owner=$liaisonId", $other);
        self::assertSame([403, 'forbidden'], [$r->getStatusCode(), self::json($r)['error']]);
        [$ambassador] = $this->auth('ambassador');
        self::assertSame(['Due old', 'Due today'], self::names($this->portion($ambassador, $liaisonId)));
        self::assertSame(404, $this->request('GET', '/api/embassy/portion?owner=00000000-0000-0000-0000-000000000000', $ambassador)->getStatusCode());
        self::assertSame(0, $this->portion($ambassador)['owned'], 'the ambassador owns nothing → the app says "add your first contacts"');
    }

    public function testHintsForMailsAndCallBacks(): void
    {
        [$liaison] = $this->auth('liaison');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]);
        $mailed = $this->org($liaison, 'Mailed', [['track' => 'live']], ['people' => [['name' => 'Lea', 'email' => 'lea@example.com']]]);
        $called = $this->org($liaison, 'Called', [['track' => 'live']]);
        $r = $this->request('POST', "/api/embassy/orgs/$mailed/mail", $liaison, ['to' => 'lea@example.com', 'track' => 'live', 'subject' => 'Top Toy', 'text' => 'Hi Lea']);
        self::assertSame(201, $r->getStatusCode(), (string) $r->getBody());
        $this->log($liaison, $called, ['kind' => 'call', 'track' => 'live', 'outcome' => 'call_back', 'next_at' => '2026-09-24']);
        $this->clock->advance(6 * self::DAY); // Sunday 27th; the mail's call is due on Monday 28th → advance one more
        $this->clock->advance(self::DAY);
        $p = $this->portion($liaison);
        self::assertSame(['Called', 'Mailed'], self::names($p));
        self::assertSame(['kind' => 'call_back', 'date' => '2026-09-24'], $p['items'][0]['hint']);
        self::assertSame(['kind' => 'mail_no_answer', 'days' => 7, 'subject' => 'Top Toy'], $p['items'][1]['hint']);
        self::assertSame(['Lea'], array_column($p['items'][1]['people'], 'name'));
        self::assertSame(['start' => '2026-09-28', 'done' => 0], ['start' => $p['week']['start'], 'done' => $p['week']['done']], 'a new week');
    }

    public function testTodayCountOnAWednesdayAndASaturday(): void
    {
        [$liaison] = $this->auth('liaison');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]);
        $ids = [];
        foreach (range(1, 12) as $n) $ids[] = $this->org($liaison, sprintf('Venue %02d', $n));
        $this->clock->advance(2 * self::DAY); // Wednesday 23rd

        $p = $this->portion($liaison);
        self::assertSame(['2026-09-23', 10, 10, 4, ['due' => 0, 'quiet' => 0, 'first' => 2]],
            [$p['today'], $p['week']['left'], count($p['items']), $p['today_count'], $p['more']], 'Wednesday: ceil(10 / 3 working days)');
        self::assertSame(['kind' => 'no_contact_yet'], $p['items'][0]['hint']);

        $this->log($liaison, $ids[0], ['kind' => 'call', 'track' => 'live', 'outcome' => 'reached']);
        $p = $this->portion($liaison);
        self::assertSame([1, 9, 1, 3], [$p['week']['done'], $p['week']['left'], $p['today_done'], $p['today_count']], 'one of today’s four done');
        self::assertNotContains('Venue 01', self::names($p));

        $this->clock->advance(3 * self::DAY); // Saturday 26th
        $p = $this->portion($liaison);
        self::assertSame(['2026-09-26', 1, 9, 9, 9], [$p['today'], $p['week']['done'], $p['week']['left'], count($p['items']), $p['today_count']], 'weekend: all that is left');

        $this->clock->advance(2 * self::DAY); // Monday 28th: a new week
        $p = $this->portion($liaison);
        self::assertSame(['2026-09-28', 0, 10, 2], [$p['week']['start'], $p['week']['done'], $p['week']['left'], $p['today_count']]);
    }

    public function testTeamOverview(): void
    {
        [$liaison, $liaisonId] = $this->auth('liaison', 'liaison@example.com');
        [$ambassador] = $this->auth('ambassador', 'amb@example.com');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]);
        $org = $this->org($liaison, 'Dachstock');
        $this->log($liaison, $org, ['kind' => 'dm', 'track' => 'live']);
        $this->request('POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'more']);

        $r = $this->request('GET', '/api/embassy/team/pace', $liaison);
        self::assertSame([403, 'forbidden'], [$r->getStatusCode(), self::json($r)['error']]);
        $items = self::json($this->request('GET', '/api/embassy/team/pace', $ambassador))['items'];
        self::assertSame(['amb@example.com', 'liaison@example.com'], array_map(fn(array $i) => $i['user']['email'], $items));
        self::assertNull($items[0]['pace']);
        $mine = $items[1];
        self::assertSame(['id' => $liaisonId, 'email' => 'liaison@example.com', 'access' => 'liaison'], $mine['user']);
        self::assertSame([15, 1], [$mine['pace']['per_week'], $mine['owned']]);
        self::assertSame([['start' => '2026-08-31', 'done' => 0], ['start' => '2026-09-07', 'done' => 0], ['start' => '2026-09-14', 'done' => 0], ['start' => '2026-09-21', 'done' => 1]],
            $mine['weeks']);
        self::assertSame(['at' => '2026-09-21T14:13:20+00:00', 'answer' => 'more', 'per_week_before' => 10, 'per_week_after' => 15], $mine['last_checkin']);
    }

    public function testCronKey(): void
    {
        self::assertSame(403, $this->request('GET', '/api/embassy/cron')->getStatusCode());
        self::assertSame(403, $this->request('GET', '/api/embassy/cron?key=wrong')->getStatusCode());
        self::assertSame(403, $this->request('POST', '/api/embassy/cron', ['Authorization' => 'Basic ' . base64_encode('cron:wrong')])->getStatusCode());
        $r = $this->request('POST', '/api/embassy/cron', ['Authorization' => 'Basic ' . base64_encode('cron:' . $this->cronKey)]);
        self::assertSame(200, $r->getStatusCode(), 'HTTP Basic: the key is the password');
        self::assertSame(['status' => 'success', 'inbox' => null, 'reminders' => ['sent' => 0, 'skipped' => 0, 'failed' => 0]], self::json($r));
        $this->cronKey = '';
        self::assertSame(404, $this->request('GET', '/api/embassy/cron?key=')->getStatusCode(), 'no key set: switched off');
    }

    public function testWeeklyReminderOncePerWeek(): void
    {
        [$liaison] = $this->auth('liaison', 'lea@example.com');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]);
        $this->org($liaison, 'Rote Fabrik', [['track' => 'live', 'next_at' => '2026-09-21']], ['people' => [['name' => 'Lea']]]);
        $this->org($liaison, 'Dachstock');

        self::assertSame(['sent' => 1, 'skipped' => 0, 'failed' => 0], $this->cron()['reminders'], 'Monday');
        $mail = $this->mailLog();
        self::assertStringContainsString('To: lea@example.com', $mail);
        self::assertStringContainsString('Subject: Embassy: 0 of 10 contacts this week', $mail);
        self::assertStringContainsString('• Rote Fabrik · Lea – follow-up due Sep 21', $mail);
        self::assertStringContainsString('• Dachstock – no contact yet – first mail', $mail);
        self::assertStringContainsString('http://embassy.test/', $mail);
        self::assertStringNotContainsString('Every 4 weeks', $mail);
        self::assertSame(1, (int) $this->db->value("SELECT COUNT(*) FROM mail_log WHERE template = 'embassy_portion' AND status = 'sent'"));

        self::assertSame(['sent' => 0, 'skipped' => 1, 'failed' => 0], $this->cron()['reminders'], 'the same day again');
        $this->clock->advance(self::DAY);
        self::assertSame(0, $this->cron()['reminders']['sent'], 'Tuesday');
        $this->clock->advance(6 * self::DAY);
        self::assertSame(1, $this->cron()['reminders']['sent'], 'next Monday');
        $this->clock->advance(8 * self::DAY);
        self::assertSame(1, $this->cron()['reminders']['sent'], 'Monday’s run was missed: the first run of the week (Tuesday)');
    }

    public function testDailyReminderSkipsTheWeekendAndSpeaksGerman(): void
    {
        [$liaison, $id] = $this->auth('liaison', 'kathi@example.com');
        $this->db->run("UPDATE users SET locale = 'de' WHERE id = ?", [$id]);
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 5, 'reminder' => 'daily']);
        foreach (range(1, 8) as $n) $this->org($liaison, "Venue $n");

        $sent = [];
        foreach (range(0, 6) as $day) { // Monday … Sunday
            $sent[] = $this->cron()['reminders']['sent'];
            self::assertSame(0, $this->cron()['reminders']['sent'], 'once a day');
            $this->clock->advance(self::DAY);
        }
        self::assertSame([1, 1, 1, 1, 1, 0, 0], $sent);
        $mail = $this->mailLog();
        self::assertStringContainsString('Subject: Embassy: 0 von 5 Kontakten diese Woche', $mail);
        self::assertStringContainsString("Für heute:\n\n• Venue 1 – noch kein Kontakt – erste Mail\n\n+7 weitere, wenn du Zeit hast.", str_replace("\r\n", "\n", $mail),
            'Monday: 5 over 5 days → 1 today, the rest as "+N more"');
    }

    public function testNoMailWhenNothingToDoUnlessTheCheckinIsDue(): void
    {
        [$liaison] = $this->auth('liaison', 'idle@example.com');
        $this->request('PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]);
        [$off] = $this->auth('liaison');
        $this->request('PUT', '/api/embassy/pace', $off, ['reminder' => 'off']);
        $this->org($off, 'Not reminded');
        // a pace row, but the role is gone
        $gone = $this->user('gone@example.com');
        $this->db->run("INSERT INTO embassy_pace (user_id, per_week, reminder, onboarded_at, asked_at, updated_at) VALUES (?, 10, 'daily', ?, ?, ?)",
            [$gone['id'], $this->clock->utc(), $this->clock->utc(), $this->clock->utc()]);
        $this->org($liaison, 'Opted out', [['track' => 'live']], ['do_not_contact' => true]);

        self::assertSame(['sent' => 0, 'skipped' => 1, 'failed' => 0], $this->cron()['reminders'], 'no items, no check-in: no mail');
        self::assertSame('', $this->mailLog());

        $this->clock->advance(28 * self::DAY); // Monday, check-in due
        self::assertSame(1, $this->cron()['reminders']['sent']);
        self::assertStringContainsString('Every 4 weeks: is your pace right? Answer on Today.', $this->mailLog());
        self::assertStringNotContainsString('gone@example.com', $this->mailLog());
    }

    public function testWithoutTheMigration(): void
    {
        [$liaison] = $this->auth('liaison');
        [$ambassador] = $this->auth('ambassador');
        $this->db->exec('DROP TABLE embassy_checkins');
        $this->db->exec('DROP TABLE embassy_pace');
        $me = $this->request('GET', '/api/embassy/me', $liaison);
        self::assertSame([200, null], [$me->getStatusCode(), self::json($me)['pace']]);
        foreach ([['PUT', '/api/embassy/pace', $liaison, ['per_week' => 10]], ['POST', '/api/embassy/pace/checkin', $liaison, ['answer' => 'fine']],
            ['GET', '/api/embassy/portion', $liaison, null], ['GET', '/api/embassy/team/pace', $ambassador, null]] as [$method, $path, $auth, $body]) {
            $r = $this->request($method, $path, $auth, $body);
            self::assertSame([503, 'not_available'], [$r->getStatusCode(), self::json($r)['error'] ?? null], "$method $path");
        }
        self::assertSame(['status' => 'success', 'inbox' => null, 'reminders' => null], $this->cron());
    }
}
