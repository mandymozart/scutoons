<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Push\NewChapterPush;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * /api/admin/push – role admin (agents/research/PWA.md §2b; admin app → Emails → Push):
 *   GET  /push               { configured, subscribers: { total, locales }, sends: [last 20] }
 *   POST /push/new-chapter   { toon, chapter, test?, dry_run? } → { notification, recipients, sent_before, configured,
 *                            + delivered, gone, failed when sent }
 * The only push there is: "a new chapter is out", sent by an admin's tap – never automatically.
 */
final class PushController
{
    public function __construct(private readonly NewChapterPush $push)
    {
    }

    public function overview(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->push->overview());
    }

    public function newChapter(Request $request, Response $response): Response
    {
        $admin = CurrentSession::of($request)->requireUser();
        return Json::response($response, 200, ['status' => 'success'] + $this->push->send(JsonBody::of($request), $admin));
    }
}
