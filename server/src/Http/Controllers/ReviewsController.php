<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Assets\AssetUsages;
use Scutoons\Domain\Content\Reviews;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Review before publish (docs/API.md "Reviews", Domain\Content\Reviews; role creator for everything, the rules decide
 * the rest):
 *   GET  /api/reviews                                { needs_review, reviewer_of, to_review, mine }
 *   POST /api/reviews          { items: [{ key, version? }], note? }   submit (mails the reviewers)
 *   POST /api/reviews/publish  { ids, note? }        a reviewer publishes the submitted versions (mails the author)
 *   POST /api/reviews/return   { ids, note? }        a reviewer sends them back (mails the author)
 */
final class ReviewsController
{
    public function __construct(private readonly Reviews $reviews, private readonly Roles $roles, private readonly AssetUsages $usages)
    {
    }

    public function overview(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->reviews->overview($this->creator($request)));
    }

    public function submit(Request $request, Response $response): Response
    {
        $user = $this->creator($request);
        return Json::response($response, 200, $this->reviews->submit($user, $this->body($request)));
    }

    /** @param array{action: string} $args */
    public function decide(Request $request, Response $response, array $args): Response
    {
        $user = $this->creator($request);
        $result = $this->reviews->decide($user, $this->body($request), $args['action'] === 'publish');
        if ($args['action'] === 'publish') foreach ($result['keys'] as $key) $this->usages->reindexEntity($key);
        return Json::response($response, 200, $result);
    }

    /** @return array<string, mixed> */
    private function creator(Request $request): array
    {
        $session = CurrentSession::of($request);
        $session->rejectCsrf();
        return $this->roles->requireContentEditor($session->user());
    }

    private function body(Request $request): \stdClass
    {
        return JsonBody::object($request) ?? throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
    }
}
