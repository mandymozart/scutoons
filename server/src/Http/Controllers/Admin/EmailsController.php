<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Domain\Mail\MailRecipients;
use Scutoons\Domain\Mail\MailRenderer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/emails – role admin (docs/API.md "Admin API" → Emails): the email module (Domain\Mail).
 *   GET /emails/templates                        { status, locales, transport, items: [{ id, audience, description, vars, sample }] }
 *   GET /emails/templates/{id}/preview?locale=   { status, id, locale, subject, text, html } rendered with the sample vars
 *   GET /emails/log                              the sent-mail log (list: sort created_at|template|status|recipient,
 *                                                filters template, status, recipient, from, to; q = part of the address)
 *   GET /emails/recipients                       the admin notification addresses { items, default, stored, updated_at, updated_by, max }
 *   PUT /emails/recipients                       { items: [email] | null } → { status, changed, …same }
 */
final class EmailsController
{
    public function __construct(
        private readonly MailRenderer $renderer,
        private readonly MailLog $log,
        private readonly MailRecipients $recipients,
        private readonly Settings $settings,
    ) {
    }

    public function templates(Request $request, Response $response): Response
    {
        $items = [];
        foreach (MailTemplates::ids() as $id) {
            $definition = MailTemplates::get($id);
            $items[] = [
                'id' => $id,
                'audience' => $definition['audience'],
                'description' => $definition['description'],
                'vars' => $definition['vars'],
                'sample' => $definition['sample'],
            ];
        }
        return Json::response($response, 200, [
            'status' => 'success',
            'locales' => MailTemplates::LOCALES,
            'transport' => $this->settings->get('MAIL_TRANSPORT'),
            'items' => $items,
        ]);
    }

    /** @param array<string, string> $args */
    public function preview(Request $request, Response $response, array $args): Response
    {
        $id = $args['id'];
        if (!MailTemplates::exists($id)) throw new ApiError(404, 'not_found', "Unknown email template $id.", ['templates' => MailTemplates::ids()]);
        $locale = $request->getQueryParams()['locale'] ?? MailTemplates::DEFAULT_LOCALE;
        if (!is_string($locale) || !in_array($locale, MailTemplates::LOCALES, true)) {
            throw new ApiError(400, 'invalid_locale', 'locale must be one of ' . implode(', ', MailTemplates::LOCALES) . '.', ['locales' => MailTemplates::LOCALES]);
        }
        $mail = $this->renderer->render($id, $locale, MailTemplates::get($id)['sample']);
        return Json::response($response, 200, ['status' => 'success', 'id' => $id, 'locale' => $locale] + $mail);
    }

    public function log(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), MailLog::SORTABLE, '-created_at', MailLog::FILTERS, 50, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return Json::response($response, 200, ['status' => 'success'] + $this->log->list($query));
    }

    public function recipients(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->recipients->state());
    }

    public function saveRecipients(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        if (!array_key_exists('items', $body)) {
            throw new ApiError(400, 'invalid_recipients', 'Send { items: [email, …] } or { items: null } for the default.');
        }
        $admin = $request->getAttribute('user');
        $by = 'admin:' . (is_array($admin) ? (string) ($admin['email'] ?? '') : '');
        $changed = $this->recipients->update($body['items'], $by);
        return Json::response($response, 200, ['status' => 'success', 'changed' => $changed] + $this->recipients->state());
    }
}
