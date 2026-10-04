<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Domain\Embassy\EmbassyInbox;
use Scutoons\Domain\Embassy\EmbassyMail;
use Scutoons\Domain\Embassy\EmbassyPace;
use Scutoons\Domain\Embassy\EmbassyReminders;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/embassy/* – the Embassy app (embassy.scutoons.com), roles liaison / ambassador (admin counts as ambassador).
 * Contract: server/docs/API.md "Embassy"; rules: Domain\Embassy\Embassy.
 *   GET    /embassy/me                          → { access, user, tracks, stages, types, stockist_types, kinds, outcomes, tiers, mail: { from, inbox }, pace }
 *                                               (pace: { per_week, reminder, onboarded_at, asked_at, checkin_due } | null = not onboarded)
 *   GET    /embassy/team                        → { items: [{ id, email, access }] }
 *   GET    /embassy/today?owner=me|all|none|<id> → { today, due: [{ lead, org, people, last_activity, overdue_days }], unscheduled }
 *   GET    /embassy/orgs?q&type&country&tier&track&stage&owner&due&quiet&sort&limit&offset → { items, total }
 *   POST   /embassy/orgs                        { …org, people?, leads?, force? } → { org, people, leads, activities } (201)
 *   GET    /embassy/orgs/{id}                   → { org, people, leads, activities }
 *   PUT    /embassy/orgs/{id}                   { …fields } → same
 *   DELETE /embassy/orgs/{id}                   (ambassador)
 *   POST   /embassy/orgs/{id}/people            { person } → { person } (201)
 *   PUT    /embassy/people/{id}                 { fields } → { person }
 *   DELETE /embassy/people/{id}                 (ambassador)
 *   PUT    /embassy/orgs/{id}/leads/{track}     { stage?, owner_id?, next_at? } → { lead }
 *   POST   /embassy/orgs/{id}/activities        { kind, track?, person_id?, outcome?, note?, stage?, next_at? } → { activity, lead } (201)
 *   POST   /embassy/orgs/{id}/mail              { to, person_id?, track?, subject, text, reply_to_activity_id?, language? } → { activity, lead } (201)
 *                                               sends from the shared mailbox (EmbassyMail) · 400 / 409 do_not_contact / 502 mail_failed
 *   GET    /embassy/inbox                       → { configured, synced_at, error }
 *   POST   /embassy/inbox/sync                  { force? } → { configured, synced_at, matched, skipped, new_activities, error, throttled }
 *                                               (EmbassyInbox; at most every 2 minutes – ambassadors may force)
 *   POST   /embassy/import                      { organisations: [ … ], dry_run? } → report (ambassador)
 *   PUT    /embassy/pace                        { per_week?, reminder? } → { pace } (EmbassyPace; the first call = the onboarding)
 *   POST   /embassy/pace/checkin                { answer: less|fine|more } → { pace }
 *   GET    /embassy/portion?owner=me|<id>       → { today, week, today_count, today_done, items, more, owned } (another's: ambassador)
 *   GET    /embassy/team/pace                   → { items: [{ user, pace, owned, weeks, last_checkin }] } (ambassador)
 *                                               pace endpoints: 503 not_available before migration 2026-10-05-embassy-pace
 *   GET|POST /embassy/cron?key=…                (no session: EMBASSY_CRON_KEY in ?key= or as the HTTP Basic password; empty
 *                                               setting → 404, wrong key → 403) → { inbox, reminders } (EmbassyReminders)
 */
final class EmbassyController
{
    public function __construct(
        private readonly Embassy $embassy,
        private readonly EmbassyMail $mail,
        private readonly EmbassyInbox $inbox,
        private readonly EmbassyPace $pace,
        private readonly EmbassyReminders $reminders,
        private readonly Settings $settings,
    ) {
    }

    public function me(Request $request, Response $response): Response
    {
        $user = $this->user($request);
        return $this->ok($response, [
            'access' => $this->embassy->access($user),
            'user' => ['id' => (string) $user['id'], 'email' => (string) $user['email']],
            'tracks' => Embassy::TRACKS, 'stages' => Embassy::STAGES, 'types' => Embassy::TYPES, 'stockist_types' => Embassy::STOCKIST_TYPES,
            'kinds' => Embassy::KINDS, 'outcomes' => Embassy::OUTCOMES, 'tiers' => Embassy::TIERS,
            'mail' => ['from' => $this->mail->from(), 'inbox' => $this->inbox->configured()],
            'pace' => $this->pace->of((string) $user['id']),
        ]);
    }

    public function team(Request $request, Response $response): Response
    {
        $this->user($request);
        return $this->ok($response, ['items' => $this->embassy->team()]);
    }

    public function today(Request $request, Response $response): Response
    {
        $owner = (string) ($request->getQueryParams()['owner'] ?? 'me');
        return $this->ok($response, $this->embassy->today($owner, $this->user($request)));
    }

    public function list(Request $request, Response $response): Response
    {
        $user = $this->user($request);
        return $this->ok($response, $this->embassy->list($request->getQueryParams(), $user));
    }

    public function create(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->embassy->create(JsonBody::of($request), $this->user($request)), 201);
    }

    /** @param array<string, string> $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->user($request);
        return $this->ok($response, $this->embassy->get($args['id']));
    }

    /** @param array<string, string> $args */
    public function update(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, $this->embassy->update($args['id'], JsonBody::of($request), $this->user($request)));
    }

    /** @param array<string, string> $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $this->embassy->delete($args['id'], $this->user($request));
        return $this->ok($response, []);
    }

    /** @param array<string, string> $args */
    public function addPerson(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['person' => $this->embassy->addPerson($args['id'], JsonBody::of($request), $this->user($request))], 201);
    }

    /** @param array<string, string> $args */
    public function updatePerson(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['person' => $this->embassy->updatePerson($args['id'], JsonBody::of($request), $this->user($request))]);
    }

    /** @param array<string, string> $args */
    public function deletePerson(Request $request, Response $response, array $args): Response
    {
        $this->embassy->deletePerson($args['id'], $this->user($request));
        return $this->ok($response, []);
    }

    /** @param array<string, string> $args */
    public function lead(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['lead' => $this->embassy->setLead($args['id'], $args['track'], JsonBody::of($request), $this->user($request))]);
    }

    /** @param array<string, string> $args */
    public function log(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, $this->embassy->log($args['id'], JsonBody::of($request), $this->user($request)), 201);
    }

    /** @param array<string, string> $args */
    public function mail(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, $this->mail->send($args['id'], JsonBody::of($request), $this->user($request)), 201);
    }

    public function inbox(Request $request, Response $response): Response
    {
        $this->user($request);
        return $this->ok($response, $this->inbox->status());
    }

    public function syncInbox(Request $request, Response $response): Response
    {
        $force = (JsonBody::of($request)['force'] ?? false) === true;
        return $this->ok($response, $this->inbox->sync($this->user($request), $force));
    }

    public function import(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->embassy->import(JsonBody::of($request), $this->user($request)));
    }

    public function setPace(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->pace->set(JsonBody::of($request), $this->user($request)));
    }

    public function checkin(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->pace->checkin(JsonBody::of($request), $this->user($request)));
    }

    public function portion(Request $request, Response $response): Response
    {
        $owner = (string) ($request->getQueryParams()['owner'] ?? 'me');
        return $this->ok($response, $this->pace->portion($owner, $this->user($request)));
    }

    public function teamPace(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->pace->team($this->user($request)));
    }

    /**
     * The host's cron job (all-inkl calls a URL and cannot set headers): the key in `?key=` or as the HTTP Basic
     * password, compared in constant time. EMBASSY_CRON_KEY empty = switched off (404).
     */
    public function cron(Request $request, Response $response): Response
    {
        $key = $this->settings->get('EMBASSY_CRON_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getQueryParams()['key'] ?? null;
        $given = is_string($given) && $given !== '' ? $given : self::basicPassword($request);
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        return $this->ok($response, $this->reminders->cron());
    }

    /** The password of an `Authorization: Basic …` header ('' = none) */
    private static function basicPassword(Request $request): string
    {
        $server = $request->getServerParams();
        if (isset($server['PHP_AUTH_PW']) && is_string($server['PHP_AUTH_PW'])) return $server['PHP_AUTH_PW'];
        $header = $request->getHeaderLine('Authorization');
        if ($header === '') $header = (string) ($server['HTTP_AUTHORIZATION'] ?? $server['REDIRECT_HTTP_AUTHORIZATION'] ?? '');
        if (!preg_match('/^Basic\s+(\S+)$/i', trim($header), $m)) return '';
        $decoded = base64_decode($m[1], true);
        if ($decoded === false || !str_contains($decoded, ':')) return '';
        return substr($decoded, strpos($decoded, ':') + 1);
    }

    /** @return array<string, mixed> the signed-in user with Embassy access (401 / 403 otherwise) */
    private function user(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        $user = $session->user();
        $this->embassy->access($user);
        /** @var array<string, mixed> $user */
        return $user;
    }

    /** @param array<string, mixed> $data */
    private function ok(Response $response, array $data, int $status = 200): Response
    {
        return Json::response($response, $status, ['status' => 'success'] + $data);
    }
}
