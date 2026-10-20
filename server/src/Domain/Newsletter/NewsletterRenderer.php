<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * Renders one newsletter for one recipient (agents/NEWSLETTER.md "Writing a newsletter") in the same frame as every
 * platform mail (templates/mail/layout.html.php), text and HTML from the same blocks. The body is plain text:
 *   - a blank line starts a new paragraph; single line breaks stay
 *   - a paragraph that is only `[Button text](https://…)` becomes a button (the text part prints the URL)
 *   - a paragraph that is only `![description](path)` becomes a picture (owner 2026-10-09): the path from the CDN root
 *     (what the asset picker gives) or an http(s) URL; the mail loads it from the public CDN, the text part prints the
 *     description. Anything else that looks like it stays text.
 *   - `{name}` = the recipient's name (empty without one), `{name|there}` = the name or "there" (also in the subject)
 *   - a paragraph that is only `{signin}` becomes the recipient's personal sign-in / sign-up button (NewsletterSignin,
 *     #65), one that is only `{gift}` the button to the newsletter's gift (Campaigns\Giveaways, #66) – left out when there
 *     is none for them
 * Under the body: why the person gets it and the unsubscribe link (also the List-Unsubscribe header, NewsletterSender).
 */
final class NewsletterRenderer
{
    private const TEXTS = [
        'de' => [
            'reason' => 'Du bekommst diese E-Mail, weil du bei SCUTOONS registriert bist oder dich für Neuigkeiten eingetragen hast.',
            'unsubscribe' => 'Keine Neuigkeiten mehr? Abmelden',
        ],
        'en' => [
            'reason' => 'You get this email because you have a SCUTOONS account or signed up for news.',
            'unsubscribe' => 'No more news? Unsubscribe',
        ],
    ];

    public function __construct(
        private readonly Templates $templates,
        private readonly UnsubscribeTokens $tokens,
        private readonly AssetStorages $storages,
        private readonly Settings $settings,
    ) {
    }

    /**
     * `![description](path)` → [description, absolute URL]; null when it isn't a picture a mail can load
     * @return array{0: string, 1: string}|null
     */
    private function picture(string $paragraph): ?array
    {
        if (!preg_match('/^!\[([^\]\n]*)\]\(([^\s)]+)\)$/', $paragraph, $m)) return null;
        [, $alt, $src] = $m;
        if (preg_match('#^https?://#i', $src)) return [$alt, $src];
        // a CDN path: plain segments only (no scheme, no .., no query)
        $path = ltrim($src, '/');
        if ($path === '' || !preg_match('#^[A-Za-z0-9._~/-]+$#', $path) || in_array('..', explode('/', $path), true)) return null;
        return [$alt, rtrim($this->storages->publicBase(), '/') . '/' . implode('/', array_map('rawurlencode', explode('/', $path)))];
    }

    /**
     * @param array{subject: string, body: string} $content the locale's subject + body
     * @param (\Closure(string): ?array{text: string, url: string})|null $button the `{signin}` / `{gift}` button (its
     *        argument: signin | gift), asked only when the body has one (it makes a login request); null = the preview's samples
     * @return array{subject: string, text: string, html: string, unsubscribe: string}
     */
    public function render(array $content, string $locale, string $email, ?string $name, ?\Closure $button = null): array
    {
        $locale = MailTemplates::locale($locale);
        $fill = static fn(string $text): string => (string) preg_replace_callback('/\{name(?:\|([^}]*))?\}/', static fn(array $m) =>
            $name !== null && $name !== '' ? $name : ($m[1] ?? ''), $text);

        $blocks = [];
        $text = '';
        $body = str_replace(["\r\n", "\r"], "\n", $fill($content['body']));
        foreach (preg_split('/\n[ \t]*\n/', trim($body)) ?: [] as $paragraph) {
            $paragraph = trim($paragraph);
            if ($paragraph === '') continue;
            if ($paragraph === '{signin}' || $paragraph === '{gift}') {
                $kind = trim($paragraph, '{}');
                $personal = match (true) {
                    $button !== null => $button($kind),
                    $kind === 'gift' => Giveaways::sample($locale, $this->settings->get('APP_URL')),
                    default => NewsletterSignin::sample($locale, $this->settings->get('APP_URL')),
                };
                if ($personal === null) continue;
                $blocks[] = ['type' => 'button', 'text' => $personal['text'], 'url' => $personal['url']];
                $text .= $personal['text'] . ': ' . $personal['url'] . "\n\n";
            } elseif (($picture = $this->picture($paragraph)) !== null) {
                [$alt, $url] = $picture;
                $blocks[] = ['type' => 'image', 'text' => $alt, 'url' => $url];
                if ($alt !== '') $text .= '[' . $alt . "]\n\n";
            } elseif (preg_match('/^\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)$/', $paragraph, $m)) {
                $blocks[] = ['type' => 'button', 'text' => $m[1], 'url' => $m[2]];
                $text .= $m[1] . ': ' . $m[2] . "\n\n";
            } else {
                $blocks[] = ['type' => str_contains($paragraph, "\n") ? 'lines' : 'p', 'text' => $paragraph];
                $text .= $paragraph . "\n\n";
            }
        }
        $texts = self::TEXTS[$locale];
        $unsubscribe = $this->tokens->url($email, $locale);
        $text .= "– SCUTOONS\n\n" . $texts['reason'] . "\n" . $texts['unsubscribe'] . ': ' . $unsubscribe . "\n";
        $html = $this->templates->render('mail/layout.html', [
            'locale' => $locale,
            'blocks' => $blocks,
            'fineprint' => [$texts['reason']],
            'unsubscribe' => ['text' => $texts['unsubscribe'], 'url' => $unsubscribe],
        ]);
        return ['subject' => trim($fill($content['subject'])), 'text' => $text, 'html' => $html, 'unsubscribe' => $unsubscribe];
    }
}
