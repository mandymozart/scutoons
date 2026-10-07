<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Newsletter\NewsletterSender;
use Scutoons\Domain\Newsletter\Suppressions;
use Scutoons\Domain\Newsletter\UnsubscribeTokens;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * The newsletter's public endpoints (no session; agents/NEWSLETTER.md, docs/API.md "Newsletter"):
 *   GET  /api/newsletter/unsubscribe?t=…   a small page (the token's language) with one "Unsubscribe" button – a GET
 *                                          changes nothing (mail scanners open links)
 *   POST /api/newsletter/unsubscribe?t=…   unsubscribes (the page's button, and the mail client's one-click button:
 *                                          List-Unsubscribe-Post, RFC 8058) → the page "You are unsubscribed"
 *   GET|POST /api/newsletter/cron?key=…    one batch (NewsletterSender) – the host's cron every few minutes; the key is
 *                                          NEWSLETTER_CRON_KEY (or the HTTP Basic password); empty key = 404
 */
final class NewsletterController
{
    public function __construct(
        private readonly UnsubscribeTokens $tokens,
        private readonly Suppressions $suppressions,
        private readonly NewsletterSender $sender,
        private readonly Templates $templates,
        private readonly Settings $settings,
    ) {
    }

    public function unsubscribePage(Request $request, Response $response): Response
    {
        $token = $request->getQueryParams()['t'] ?? null;
        $data = $this->tokens->read($token);
        return $this->page($response, $data === null ? 'invalid' : 'confirm', $data['locale'] ?? 'en', $data['email'] ?? null, is_string($token) ? $token : '');
    }

    public function unsubscribe(Request $request, Response $response): Response
    {
        $token = $request->getQueryParams()['t'] ?? null;
        $data = $this->tokens->read($token);
        if ($data === null) return $this->page($response, 'invalid', 'en', null, '', 400);
        $this->suppressions->add($data['email'], 'unsubscribe', 'link');
        return $this->page($response, 'done', $data['locale'], $data['email'], '');
    }

    public function cron(Request $request, Response $response): Response
    {
        $key = $this->settings->get('NEWSLETTER_CRON_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getQueryParams()['key'] ?? null;
        $given = is_string($given) && $given !== '' ? $given : (string) ($request->getServerParams()['PHP_AUTH_PW'] ?? '');
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        return Json::response($response, 200, ['status' => 'success', 'result' => $this->sender->batch()]);
    }

    private function page(Response $response, string $state, string $locale, ?string $email, string $token, int $status = 200): Response
    {
        $html = $this->templates->render('newsletter/unsubscribe.html', ['state' => $state, 'locale' => $locale, 'email' => $email, 'token' => $token]);
        $response->getBody()->write($html);
        return $response->withStatus($status)->withHeader('Content-Type', 'text/html; charset=utf-8')
            ->withHeader('Cache-Control', 'no-store')->withHeader('X-Robots-Tag', 'noindex');
    }
}
