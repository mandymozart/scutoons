<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Psr\Http\Message\ResponseInterface;
use Scutoons\Domain\Admin\Proposals;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Content\EntityKey;
use Scutoons\Domain\Content\EntityStore;
use Scutoons\Domain\Push\PushSender;
use Scutoons\Domain\Push\PushSubscriptions;
use Scutoons\Domain\Push\TeamPush;
use Scutoons\Domain\Push\WebPushCrypto;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\AssetStorage\HttpClient;
use Scutoons\Kernel;
use Scutoons\Tests\TestCase;
use Slim\Psr7\Factory\ServerRequestFactory;

/**
 * Team push (agents/research/PWA-APPS.md §5, owner 2026-10-18): each kind to its own tool's browsers, never to the
 * person who made the change, once per thing, the person's own switch, the drafts run from the cron, the admin's test.
 */
final class TeamPushTest extends TestCase
{
    private const UA_KEY = 'BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4';
    private const AUTH = 'BTBZMqHH6r4Tts7J_aSIgg';

    /** @var array{public: string, private: string} */
    private static array $keys;
    /** @var object{sent: list<array{url: string, body: string}>} */
    private object $push;
    /** how many sends `received()` has handed out already */
    private int $seen = 0;

    protected function settings(): array
    {
        self::$keys ??= WebPushCrypto::generateKeys();
        return ['VAPID_PUBLIC_KEY' => self::$keys['public'], 'VAPID_PRIVATE_KEY' => self::$keys['private'], 'PUSH_CRON_KEY' => 'cron-key'] + parent::settings();
    }

    protected function setUp(): void
    {
        parent::setUp();
        $this->push = new class implements HttpClient {
            /** @var list<array{url: string, body: string}> */
            public array $sent = [];

            public function send(string $method, string $url, array $headers = [], ?string $body = null, ?string $bodyFile = null, ?string $sinkFile = null): array
            {
                $this->sent[] = ['url' => $url, 'body' => (string) $body];
                return ['status' => 201, 'body' => '', 'headers' => []];
            }
        };
        $this->get(PushSender::class)->useHttp($this->push);
    }

    /**
     * a user with roles and (optionally) a browser in an app – the endpoint names the person
     * @param list<string> $roles
     */
    private function person(string $name, array $roles = [], ?string $app = null): string
    {
        $id = (string) $this->user("$name@example.com")['id'];
        foreach ($roles as $role) $this->get(Roles::class)->grant($id, $role);
        if ($app !== null) $this->browser($id, $app, $name);
        return $id;
    }

    private function browser(string $userId, string $app, string $name): void
    {
        $endpoint = "https://fcm.googleapis.com/fcm/send/$name-$app";
        $this->db->run('INSERT INTO push_subscriptions (id, endpoint, p256dh, auth, user_id, locale, app, failures, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)',
            [hash('sha256', $endpoint), $endpoint, self::UA_KEY, self::AUTH, $userId, 'de', $app, $this->clock->utc(), $this->clock->utc()]);
    }

    /** @return list<string> who got something since the last call (the endpoint's name part) */
    private function received(): array
    {
        $new = array_slice($this->push->sent, $this->seen);
        $this->seen = count($this->push->sent);
        $names = array_map(static fn(array $s): string => (string) preg_replace('~^.*/send/~', '', $s['url']), $new);
        sort($names);
        return $names;
    }

    private function reviewer(string $userId, string $toon = 't'): void
    {
        $this->db->run("INSERT INTO content_review_rules (toon, user_id, rule, created_at, created_by) VALUES (?, ?, 'reviewer', ?, NULL)", [$toon, $userId, $this->clock->utc()]);
    }

    /** a draft of chapter 1 of toon `t` by `author`, saved `minutesAgo` minutes ago */
    private function draft(string $author, int $minutesAgo, string $path = 'chapter1.json'): int
    {
        $key = 'chapter:t:' . (int) preg_replace('/\D/', '', $path);
        return $this->get(EntityStore::class)->insert(EntityKey::parse($key), '{"title":"x","groups":[]}', $author, 'draft', null, $this->clock->utc(-$minutesAgo * 60));
    }

    // ── preferences ──────────────────────────────────────────────────────────────────────────────────────

    public function testTheKindsFollowTheRoles(): void
    {
        $team = $this->get(TeamPush::class);
        $creator = ['id' => $this->person('cora', [Roles::CREATOR])];
        $admin = ['id' => $this->person('ada', [Roles::ADMIN])];
        $liaison = ['id' => $this->person('lia', ['liaison'])];
        self::assertSame(['creator.reviews', 'creator.drafts'], $team->kindsFor($creator, 'creator'));
        self::assertSame([], $team->kindsFor($creator, 'admin'));
        self::assertSame(['admin.proposals'], $team->kindsFor($admin, 'admin'));
        self::assertSame(['creator.reviews', 'creator.drafts'], $team->kindsFor($admin, 'creator'), 'admins may use the creator');
        self::assertSame(['embassy.due', 'embassy.mail'], $team->kindsFor($liaison, 'embassy'));

        $prefs = $team->prefs($liaison, 'embassy');
        self::assertSame(self::$keys['public'], $prefs['key']);
        self::assertSame([['kind' => 'embassy.due', 'on' => true, 'push' => true], ['kind' => 'embassy.mail', 'on' => true, 'push' => false]], $prefs['kinds']);
        $team->setPref($liaison, 'embassy.mail', false);
        self::assertFalse($team->on((string) $liaison['id'], 'embassy.mail'));
        self::assertTrue($team->on((string) $liaison['id'], 'embassy.due'));
        $this->expectExceptionMessage('Not for this account.');
        $team->setPref($creator, 'admin.proposals', false);
    }

    // ── the kinds ────────────────────────────────────────────────────────────────────────────────────────

    public function testANewProposalGoesToTheOtherAdminsInTheAdminAppOnce(): void
    {
        $author = $this->person('ada', [Roles::ADMIN], 'admin');
        $this->person('bob', [Roles::ADMIN], 'admin');
        $this->person('cleo', [Roles::ADMIN], 'creator'); // an admin without a browser in the admin app
        $this->person('cora', [Roles::CREATOR], 'admin'); // not an admin
        $off = $this->person('dan', [Roles::ADMIN], 'admin');
        $this->get(TeamPush::class)->setPref(['id' => $off], 'admin.proposals', false);

        $proposal = $this->get(Proposals::class)->create(['title' => 'Neue Edition', 'changes' => [['type' => 'catalog']]], ['id' => $author, 'email' => 'ada@example.com']);
        self::assertSame(['bob-admin'], $this->received(), 'never the author, only the admin app, not when switched off');
        $this->get(TeamPush::class)->proposalCreated($proposal, ['id' => $author]);
        self::assertSame([], $this->received(), 'once per proposal');
        self::assertSame('admin.proposals', $this->db->value('SELECT kind FROM push_sends ORDER BY id DESC LIMIT 1'));
    }

    public function testAPublishingRequestGoesToTheReviewersNeverTheAuthor(): void
    {
        $author = $this->person('xena', [Roles::CREATOR], 'creator');
        $reviewer = $this->person('rita', [Roles::CREATOR], 'creator');
        $team = $this->get(TeamPush::class);
        $team->reviewRequested([$reviewer, $author], ['t'], 2, [7, 8], ['id' => $author, 'email' => 'xena@example.com']);
        self::assertSame(['rita-creator'], $this->received());
        $team->reviewRequested([$reviewer], ['t'], 2, [7, 8], ['id' => $author]);
        self::assertSame([], $this->received(), 'the same request once');
        $team->reviewRequested([$reviewer], ['t'], 1, [9], ['id' => $author]);
        self::assertSame(['rita-creator'], $this->received(), 'a new request again');
    }

    public function testDraftsSomeoneElseSavedGoOnlyToTheToonsReviewers(): void
    {
        $xena = $this->person('xena', [Roles::CREATOR], 'creator');
        $rita = $this->person('rita', [Roles::CREATOR], 'creator');
        $this->person('carl', [Roles::CREATOR], 'creator'); // a creator, not a reviewer of t
        $star = $this->person('sam', [Roles::ADMIN], 'creator');
        $this->reviewer($rita);
        $this->reviewer($star, '*');
        $this->reviewer($xena);
        $team = $this->get(TeamPush::class);

        $this->draft($xena, 5); // still being saved
        self::assertSame(['toons' => 0, 'notified' => 0, 'delivered' => 0], $team->cron());
        $this->draft($xena, 60, 'chapter2.json');
        $result = $team->cron();
        self::assertSame(['rita-creator', 'sam-creator'], $this->received(), 'the reviewers (also *), never xena herself, not carl');
        self::assertSame(2, $result['notified']);
        $team->cron();
        self::assertSame([], $this->received(), 'told once');

        // rita's own draft: only the others hear of it
        $this->draft($rita, 45, 'chapter3.json');
        $team->cron();
        self::assertSame(['sam-creator', 'xena-creator'], $this->received());

        // a draft with an open publishing request is the reviews kind's
        $id = $this->draft($xena, 90, 'chapter4.json');
        $this->db->run("INSERT INTO content_reviews (entity_key, toon, version_id, author, note, state, created_at) VALUES ('chapter:t:4', 't', ?, ?, NULL, 'pending', ?)", [$id, $xena, $this->clock->utc()]);
        $team->cron();
        self::assertSame([], $this->received());
    }

    public function testTheEmbassyReminderOncePerDay(): void
    {
        $lia = $this->person('lia', ['liaison'], 'embassy');
        $team = $this->get(TeamPush::class);
        self::assertTrue($team->embassyDue($lia, 3, true, '2026-10-18'));
        self::assertFalse($team->embassyDue($lia, 3, true, '2026-10-18'));
        self::assertTrue($team->embassyDue($lia, 1, true, '2026-10-19'));
        self::assertFalse($team->embassyDue($lia, 0, true, '2026-10-20'), 'nothing due: nothing');
        self::assertSame(['lia-embassy', 'lia-embassy'], $this->received());
        $payload = $this->db->value('SELECT COUNT(*) FROM push_notified WHERE user_id = ? AND kind = ?', [$lia, 'embassy.due']);
        self::assertSame(2, (int) $payload);
    }

    public function testTheReadersNewChapterNeverReachesATool(): void
    {
        $ada = $this->person('ada', [Roles::ADMIN], 'admin');
        $this->browser($ada, 'viewer', 'ada');
        $recipients = $this->get(PushSubscriptions::class)->recipients(null);
        self::assertSame(['https://fcm.googleapis.com/fcm/send/ada-viewer'], array_map(static fn(array $r) => $r['endpoint'], $recipients));
    }

    // ── HTTP ────────────────────────────────────────────────────────────────────────────────────────────

    /** @param array<string, mixed>|null $body */
    private function request(string $method, string $uri, ?array $body = null, ?string $token = null): ResponseInterface
    {
        $request = (new ServerRequestFactory())->createServerRequest($method, "http://localhost$uri", ['REMOTE_ADDR' => '127.0.0.1']);
        if ($token !== null) $request = $request->withHeader('Authorization', "Bearer $token");
        if ($body !== null) {
            $request = $request->withHeader('Content-Type', 'application/json');
            $request->getBody()->write((string) json_encode($body));
            $request->getBody()->rewind();
        }
        return Kernel::app($this->container)->handle($request);
    }

    /** @return array<string, mixed> */
    private static function json(ResponseInterface $response): array
    {
        return (array) json_decode((string) $response->getBody(), true);
    }

    public function testThePrefsTheCronAndTheAdminsTest(): void
    {
        $ada = $this->person('ada', [Roles::ADMIN]);
        $token = $this->get(Sessions::class)->create($ada);

        self::assertSame(401, $this->request('GET', '/api/push/prefs?app=admin')->getStatusCode());
        self::assertSame(400, $this->request('GET', '/api/push/prefs?app=shop', null, $token)->getStatusCode());
        $prefs = self::json($this->request('GET', '/api/push/prefs?app=admin', null, $token));
        self::assertSame([['kind' => 'admin.proposals', 'on' => true, 'push' => true]], $prefs['kinds']);
        $set = self::json($this->request('PUT', '/api/push/prefs', ['kind' => 'admin.proposals', 'on' => false], $token));
        self::assertFalse($set['kinds'][0]['on']);

        // subscribing in a tool remembers the app
        $sub = $this->request('POST', '/api/push/subscribe', ['subscription' => ['endpoint' => 'https://fcm.googleapis.com/fcm/send/ada-x', 'keys' => ['p256dh' => self::UA_KEY, 'auth' => self::AUTH]], 'locale' => 'en', 'app' => 'admin'], $token);
        self::assertSame(200, $sub->getStatusCode());
        self::assertSame('admin', $this->db->value("SELECT app FROM push_subscriptions WHERE endpoint LIKE '%ada-x'"));
        self::assertSame(400, $this->request('POST', '/api/push/subscribe', ['subscription' => ['endpoint' => 'https://fcm.googleapis.com/fcm/send/y', 'keys' => ['p256dh' => self::UA_KEY, 'auth' => self::AUTH]], 'app' => 'shop'])->getStatusCode());

        // the admin's test: to their own admin browser, also with the kind switched off
        $test = self::json($this->request('POST', '/api/admin/push/team-test', ['kind' => 'admin.proposals'], $token));
        self::assertSame(1, $test['delivered']);
        self::assertSame(['ada-x'], $this->received());
        self::assertSame(409, $this->request('POST', '/api/admin/push/team-test', ['kind' => 'creator.drafts'], $token)->getStatusCode(), 'no creator browser');

        // the cron: a key
        self::assertSame(403, $this->request('GET', '/api/push/cron?key=wrong')->getStatusCode());
        $cron = self::json($this->request('GET', '/api/push/cron?key=cron-key'));
        self::assertSame(['toons' => 0, 'notified' => 0, 'delivered' => 0], $cron['result']);
    }
}
