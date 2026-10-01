<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/embassy/* – the Embassy app (embassy.scutoons.com), roles liaison / ambassador (admin counts as ambassador).
 * Contract: server/docs/API.md "Embassy"; rules: Domain\Embassy\Embassy.
 *   GET    /embassy/me                          → { access, user, tracks, stages, types, kinds, outcomes, tiers }
 *   GET    /embassy/team                        → { items: [{ id, email, access }] }
 *   GET    /embassy/today?owner=me|all|none|<id> → { today, due: [{ lead, org, people, last_activity, overdue_days }], unscheduled }
 *   GET    /embassy/orgs?q&type&country&tier&track&stage&owner&due&sort&limit&offset → { items, total }
 *   POST   /embassy/orgs                        { …org, people?, leads?, force? } → { org, people, leads, activities } (201)
 *   GET    /embassy/orgs/{id}                   → { org, people, leads, activities }
 *   PUT    /embassy/orgs/{id}                   { …fields } → same
 *   DELETE /embassy/orgs/{id}                   (ambassador)
 *   POST   /embassy/orgs/{id}/people            { person } → { person } (201)
 *   PUT    /embassy/people/{id}                 { fields } → { person }
 *   DELETE /embassy/people/{id}                 (ambassador)
 *   PUT    /embassy/orgs/{id}/leads/{track}     { stage?, owner_id?, next_at? } → { lead }
 *   POST   /embassy/orgs/{id}/activities        { kind, track?, person_id?, outcome?, note?, stage?, next_at? } → { activity, lead } (201)
 *   POST   /embassy/import                      { organisations: [ … ], dry_run? } → report (ambassador)
 */
final class EmbassyController
{
    public function __construct(private readonly Embassy $embassy)
    {
    }

    public function me(Request $request, Response $response): Response
    {
        $user = $this->user($request);
        return $this->ok($response, [
            'access' => $this->embassy->access($user),
            'user' => ['id' => (string) $user['id'], 'email' => (string) $user['email']],
            'tracks' => Embassy::TRACKS, 'stages' => Embassy::STAGES, 'types' => Embassy::TYPES,
            'kinds' => Embassy::KINDS, 'outcomes' => Embassy::OUTCOMES, 'tiers' => Embassy::TIERS,
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

    public function import(Request $request, Response $response): Response
    {
        return $this->ok($response, $this->embassy->import(JsonBody::of($request), $this->user($request)));
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
