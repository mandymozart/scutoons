<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * Renders one newsletter for one recipient (agents/NEWSLETTER.md "Writing a newsletter") in the same frame as every
 * platform mail (templates/mail/layout.html.php), text and HTML from the same blocks. The body is plain text:
 *   - a blank line starts a new paragraph; single line breaks stay
 *   - a paragraph that is only `[Button text](https://…)` becomes a button (the text part prints the URL)
 *   - `{name}` = the recipient's name (empty without one), `{name|there}` = the name or "there" (also in the subject)
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

    public function __construct(private readonly Templates $templates, private readonly UnsubscribeTokens $tokens)
    {
    }

    /**
     * @param array{subject: string, body: string} $content the locale's subject + body
     * @return array{subject: string, text: string, html: string, unsubscribe: string}
     */
    public function render(array $content, string $locale, string $email, ?string $name): array
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
            if (preg_match('/^\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)$/', $paragraph, $m)) {
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
