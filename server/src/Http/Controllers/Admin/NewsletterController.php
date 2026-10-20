<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Newsletter\Audiences;
use Scutoons\Domain\Newsletter\Campaigns;
use Scutoons\Domain\Newsletter\Contacts;
use Scutoons\Domain\Newsletter\NewsletterRenderer;
use Scutoons\Domain\Newsletter\NewsletterSignin;
use Scutoons\Domain\Newsletter\NewsletterSender;
use Scutoons\Domain\Newsletter\Suppressions;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Mail\Transport;
use Throwable;

/**
 * /api/admin/newsletter – role admin (docs/API.md "Admin API" → Newsletter; agents/NEWSLETTER.md):
 *   GET    /newsletter/status                       { transport, cron, per_hour, sent_last_hour, queued }
 *   GET    /newsletter/audiences?selected=a,b       { items: [{ id, kind, source, count, missing_locale }], selected?: { count,
 *                                                missing_locale, locales: { de: n, en: n, none: n } } – the choice together }
 *   GET    /newsletter/contacts                     list (sort created_at|email|source|locale|name; filters source, locale = de|en|none; q)
 *   POST   /newsletter/contacts/import              { text, source, consented_at?, locale?, dry_run? } → { rows, counts, added }
 *   PUT    /newsletter/contacts                     { ids, locale?, source? } → { changed }
 *   POST   /newsletter/contacts/delete              { ids } → { deleted }
 *   PUT    /newsletter/contacts/{id}                { name?, locale? } → { item }
 *   GET    /newsletter/suppressions                 list (sort created_at|email|reason; filter reason; q)
 *   POST   /newsletter/suppressions                 { email } – block by hand
 *   POST   /newsletter/suppressions/remove          { email } – mail it again
 *   GET    /newsletter/campaigns                    { items }
 *   POST   /newsletter/campaigns                    { title, audiences?, subject_de?, subject_en?, body_de?, body_en? } → { item }
 *   GET|PUT|DELETE /newsletter/campaigns/{id}
 *   POST   /newsletter/campaigns/{id}/preview       { locale, subject?, body? } → { subject, text, html } (unsaved text allowed)
 *   POST   /newsletter/campaigns/{id}/test          { locale } → sends it to the signed-in admin
 *   POST   /newsletter/campaigns/{id}/start|pause|resume → { item }
 *   GET    /newsletter/campaigns/{id}/recipients    list (sort email|status|sent_at|locale; filters status, locale, source; q)
 *   POST   /newsletter/send                         one batch now (NewsletterSender::batch) → { result, items }
 *   GET    /newsletter/campaigns/{id}/gift?selected=a,b  who of the recipients (the saved audiences, or the unsaved choice)
 *                                                gets what from its gift (#66) { reach: { members, product_code, email_only } }
 */
final class NewsletterController
{
    public function __construct(
        private readonly Campaigns $campaigns,
        private readonly Contacts $contacts,
        private readonly Audiences $audiences,
        private readonly Suppressions $suppressions,
        private readonly NewsletterRenderer $renderer,
        private readonly NewsletterSignin $signin,
        private readonly Giveaways $giveaways,
        private readonly NewsletterSender $sender,
        private readonly Transport $transport,
        private readonly MailLog $log,
        private readonly RuntimeSettings $runtime,
        private readonly Settings $settings,
        private readonly Database $db,
        private readonly Clock $clock,
    ) {
    }

    public function status(Request $request, Response $response): Response
    {
        return $this->ok($response, [
            'transport' => $this->transport->name(),
            'cron' => $this->settings->get('NEWSLETTER_CRON_KEY') !== '',
            'per_hour' => $this->runtime->newsletterPerHour(),
            'sent_last_hour' => (int) $this->db->value("SELECT COUNT(*) FROM mail_log WHERE status = 'sent' AND created_at >= ?", [$this->clock->utc(-3600)]),
            'queued' => (int) $this->db->value("SELECT COUNT(*) FROM mail_campaign_recipients r JOIN mail_campaigns c ON c.id = r.campaign_id
                WHERE c.status = 'sending' AND r.status = 'queued'"),
        ]);
    }

    public function audiences(Request $request, Response $response): Response
    {
        $data = ['items' => $this->audiences->all()];
        $selected = $request->getQueryParams()['selected'] ?? null;
        if (is_string($selected)) {
            // the recipients of this choice together (each address once) – what "Start" would fix
            $recipients = $this->audiences->recipients(Audiences::check($selected === '' ? [] : explode(',', $selected)));
            $data['selected'] = [
                'count' => count($recipients),
                'missing_locale' => count(array_filter($recipients, fn(array $r) => $r['locale'] === null)),
                'locales' => array_count_values(array_map(fn(array $r) => $r['locale'] ?? 'none', $recipients)),
            ];
        }
        return $this->ok($response, $data);
    }

    public function contacts(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), Contacts::SORTABLE, '-created_at', Contacts::FILTERS, 100, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return $this->ok($response, $this->contacts->list($query) + ['sources' => $this->contacts->sources()]);
    }

    public function import(Request $request, Response $response): Response
    {
        $body = JsonBody::of($request);
        return $this->ok($response, $this->contacts->import($body['text'] ?? null, $body['source'] ?? null, $body['consented_at'] ?? null,
            $body['locale'] ?? null, ($body['dry_run'] ?? false) === true, $this->by($request)));
    }

    public function updateContacts(Request $request, Response $response): Response
    {
        return $this->ok($response, ['changed' => $this->contacts->updateMany(JsonBody::of($request))]);
    }

    public function deleteContacts(Request $request, Response $response): Response
    {
        return $this->ok($response, ['deleted' => $this->contacts->delete(JsonBody::of($request)['ids'] ?? null)]);
    }

    /** @param array<string, string> $args */
    public function updateContact(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->contacts->update((int) $args['id'], JsonBody::of($request))]);
    }

    public function suppressions(Request $request, Response $response): Response
    {
        $query = ListQuery::from($request->getQueryParams(), Suppressions::SORTABLE, '-created_at', Suppressions::FILTERS, 100, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return $this->ok($response, $this->suppressions->list($query));
    }

    public function suppress(Request $request, Response $response): Response
    {
        $email = $this->email(JsonBody::of($request));
        return $this->ok($response, ['added' => $this->suppressions->add($email, 'manual', $this->by($request))]);
    }

    public function unsuppress(Request $request, Response $response): Response
    {
        return $this->ok($response, ['removed' => $this->suppressions->remove($this->email(JsonBody::of($request)))]);
    }

    public function campaigns(Request $request, Response $response): Response
    {
        return $this->ok($response, ['items' => $this->campaigns->list()]);
    }

    public function create(Request $request, Response $response): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->create(JsonBody::of($request), $this->by($request))], 201);
    }

    /** @param array<string, string> $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->get((int) $args['id'])]);
    }

    /** @param array<string, string> $args */
    public function update(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->update((int) $args['id'], JsonBody::of($request), $this->by($request))]);
    }

    /** @param array<string, string> $args */
    public function delete(Request $request, Response $response, array $args): Response
    {
        $this->campaigns->delete((int) $args['id']);
        return $this->ok($response, []);
    }

    /**
     * The newsletter's gift and who gets what (the editor shows it: members sign in, QR owners sign up with their product
     * code, email-only recipients get a personal promo code – owner: "das das in der ui klar wird")
     * @param array<string, string> $args
     */
    public function gift(Request $request, Response $response, array $args): Response
    {
        $item = $this->campaigns->get((int) $args['id']);
        // ?selected=a,b = the editor's unsaved choice of audiences (as GET /newsletter/audiences)
        $selected = $request->getQueryParams()['selected'] ?? null;
        $chosen = is_string($selected) ? Audiences::check(array_values(array_filter(explode(',', $selected)))) : $item['audiences'];
        $emails = array_column($chosen !== [] ? $this->audiences->recipients($chosen) : [], 'email');
        return $this->ok($response, ['reach' => $this->giveaways->reach($emails)]);
    }

    /** @param array<string, string> $args */
    public function preview(Request $request, Response $response, array $args): Response
    {
        $campaign = $this->campaigns->get((int) $args['id']);
        $body = JsonBody::of($request);
        $locale = Contacts::locale($body['locale'] ?? null, false);
        $content = [
            'subject' => is_string($body['subject'] ?? null) ? $body['subject'] : (string) $campaign["subject_$locale"],
            'body' => is_string($body['body'] ?? null) ? $body['body'] : (string) $campaign["body_$locale"],
        ];
        $admin = $this->admin($request);
        $mail = $this->renderer->render($content, (string) $locale, $admin['email'], $admin['name']);
        return $this->ok($response, ['locale' => $locale, 'subject' => $mail['subject'], 'text' => $mail['text'], 'html' => $mail['html']]);
    }

    /** @param array<string, string> $args */
    public function test(Request $request, Response $response, array $args): Response
    {
        $campaign = $this->campaigns->get((int) $args['id']);
        $locale = (string) Contacts::locale(JsonBody::of($request)['locale'] ?? null, false);
        $content = ['subject' => (string) $campaign["subject_$locale"], 'body' => (string) $campaign["body_$locale"]];
        if (trim($content['subject']) === '' || trim($content['body']) === '') {
            throw new ApiError(409, 'missing_text', "Subject and text in $locale first.", ['locale' => $locale]);
        }
        $admin = $this->admin($request);
        // the admin's own `{signin}` button: a staff account's = the account page (NewsletterSignin)
        $mail = $this->renderer->render($content, $locale, $admin['email'], $admin['name'], fn(string $kind, ?string $to = null) => $kind === 'gift'
            ? $this->giveaways->buttonFor($this->db->one('SELECT * FROM mail_campaigns WHERE id = ?', [(int) $args['id']]) ?? [], $admin['email'], $locale)
            : $this->signin->button($admin['email'], $locale, $to));
        try {
            $this->transport->deliverWithHeaders($admin['email'], '[Test] ' . $mail['subject'], $mail['text'], $mail['html'],
                ['List-Unsubscribe: <' . $mail['unsubscribe'] . '>', 'List-Unsubscribe-Post: List-Unsubscribe=One-Click']);
        } catch (Throwable $error) {
            $this->log->record(NewsletterSender::TEMPLATE, $admin['email'], $locale, 'failed', $this->transport->name(), $error->getMessage());
            throw new ApiError(502, 'mail_failed', 'The test mail could not be sent: ' . $error->getMessage());
        }
        $this->log->record(NewsletterSender::TEMPLATE, $admin['email'], $locale, 'sent', $this->transport->name(), null);
        return $this->ok($response, ['to' => $admin['email'], 'locale' => $locale]);
    }

    /** @param array<string, string> $args */
    public function start(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->start((int) $args['id'], $this->by($request))]);
    }

    /** @param array<string, string> $args */
    public function pause(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->pause((int) $args['id'], $this->by($request))]);
    }

    /** @param array<string, string> $args */
    public function resume(Request $request, Response $response, array $args): Response
    {
        return $this->ok($response, ['item' => $this->campaigns->resume((int) $args['id'], $this->by($request))]);
    }

    /** @param array<string, string> $args */
    public function recipients(Request $request, Response $response, array $args): Response
    {
        $query = ListQuery::from($request->getQueryParams(), Campaigns::RECIPIENT_SORTABLE, 'email', Campaigns::RECIPIENT_FILTERS, 100, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        return $this->ok($response, $this->campaigns->recipients((int) $args['id'], $query));
    }

    public function send(Request $request, Response $response): Response
    {
        return $this->ok($response, ['result' => $this->sender->batch(), 'items' => $this->campaigns->list()]);
    }

    /** @param array<string, mixed> $data */
    private function ok(Response $response, array $data, int $status = 200): Response
    {
        return Json::response($response, $status, ['status' => 'success'] + $data);
    }

    /** @param array<mixed> $body */
    private function email(array $body): string
    {
        $email = \Scutoons\Domain\Accounts\Users::normalizeEmail($body['email'] ?? null);
        if ($email === null) throw new ApiError(400, 'invalid_email', 'email: a valid address.');
        return $email;
    }

    /** @return array{email: string, name: ?string} the signed-in admin */
    private function admin(Request $request): array
    {
        $user = $request->getAttribute('user');
        $email = is_array($user) ? (string) ($user['email'] ?? '') : '';
        if ($email === '') throw new ApiError(401, 'unauthorized', 'Sign in first.');
        $name = is_array($user) && is_string($user['display_name'] ?? null) && $user['display_name'] !== '' ? $user['display_name'] : null;
        return ['email' => $email, 'name' => $name];
    }

    private function by(Request $request): string
    {
        $user = $request->getAttribute('user');
        return 'admin:' . (is_array($user) ? (string) ($user['email'] ?? '') : '');
    }
}
