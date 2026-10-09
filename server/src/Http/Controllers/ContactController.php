<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Contact\ContactForm;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * POST /api/contact { topic, email, name?, code?, message, locale?, page?, website, elapsed } → { status: success }
 * The public contact form (Domain\Contact\ContactForm); a per-IP limit per hour (config/routes.php). A message
 * dropped as spam answers success too.
 */
final class ContactController
{
    public function __construct(private readonly ContactForm $form)
    {
    }

    public function send(Request $request, Response $response): Response
    {
        $this->form->submit(JsonBody::of($request), CurrentSession::of($request)->user());
        return Json::response($response, 200, ['status' => 'success']);
    }
}
