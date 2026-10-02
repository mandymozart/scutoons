<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use InvalidArgumentException;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * Renders a registered email (MailTemplates) in one locale: subject, plain text and HTML from the same blocks, so both
 * parts always say the same. Texts from templates/mail/texts.php ({var} placeholders filled), HTML frame
 * templates/mail/layout.html.php. Pure – sends nothing (the admin preview uses it with the sample vars).
 *
 * Plain text: each block + a blank line (p = the text, button = the URL, code = the label, a blank line, the code
 * indented by four spaces), then the fine print lines and the signature ("– SCUTOONS").
 */
final class MailRenderer
{
    /** @var array<string, array<string, array<string, string>>>|null */
    private ?array $texts = null;

    public function __construct(private readonly Templates $templates)
    {
    }

    /**
     * @param array<string, mixed> $vars
     * @return array{subject: string, text: string, html: string}
     * @throws InvalidArgumentException unknown template, a missing or wrong variable, a missing text
     */
    public function render(string $id, string $locale, array $vars): array
    {
        $definition = MailTemplates::get($id);
        $locale = MailTemplates::locale($locale);
        $vars = self::checkVars($id, $definition['vars'], $vars);

        $placeholders = [];
        foreach ($vars as $name => $value) $placeholders['{' . $name . '}'] = is_bool($value) ? ($value ? '1' : '') : (string) $value;
        $texts = $this->texts()[$locale][$id] ?? throw new InvalidArgumentException("No $locale texts for email template $id");
        $t = static function (string $key) use ($texts, $placeholders, $id, $locale): string {
            $text = $texts[$key] ?? throw new InvalidArgumentException("Missing text $locale.$id.$key (templates/mail/texts.php)");
            return strtr($text, $placeholders);
        };

        $blocks = [];
        $text = '';
        foreach ($definition['blocks'] as $block) {
            $condition = $block['if'] ?? null;
            if ($condition !== null && in_array($vars[$condition] ?? null, [null, false, ''], true)) continue;
            [$type, $key] = [(string) $block[0], (string) $block[1]];
            switch ($type) {
                case 'p':
                    $blocks[] = ['type' => 'p', 'text' => $t($key)];
                    $text .= $t($key) . "\n\n";
                    break;
                case 'button':
                    $url = (string) $vars[$block['url']];
                    $blocks[] = ['type' => 'button', 'text' => $t($key), 'url' => $url];
                    $text .= $url . "\n\n";
                    break;
                case 'code':
                    $value = (string) $vars[$block['value']];
                    $blocks[] = ['type' => 'code', 'text' => $t($key), 'value' => $value];
                    $text .= $t($key) . "\n\n    " . $value . "\n\n";
                    break;
                default:
                    throw new InvalidArgumentException("Unknown block type $type in email template $id");
            }
        }
        $fineprint = array_map($t, $definition['fineprint']);
        $signature = $this->texts()[$locale]['_layout']['signature'] ?? '– SCUTOONS';
        $text .= ($fineprint !== [] ? implode("\n", $fineprint) . "\n\n" : '') . $signature . "\n";

        $html = $this->templates->render('mail/layout.html', ['locale' => $locale, 'blocks' => $blocks, 'fineprint' => $fineprint]);
        return ['subject' => $t('subject'), 'text' => $text, 'html' => $html];
    }

    /**
     * The template's variables, checked: required ones present, scalars only; optional ones filled with null / false
     * @param array<string, string> $types
     * @param array<string, mixed> $vars
     * @return array<string, scalar|null>
     */
    private static function checkVars(string $id, array $types, array $vars): array
    {
        $clean = [];
        foreach ($types as $name => $type) {
            $value = $vars[$name] ?? null;
            if ($value === null) {
                if ($type === 'bool') $value = false;
                elseif (!str_starts_with($type, '?')) throw new InvalidArgumentException("Email template $id needs the variable $name");
            } elseif (!is_scalar($value)) {
                throw new InvalidArgumentException("Email template $id: $name must be a $type");
            }
            $clean[$name] = $type === 'bool' ? (bool) $value : $value;
        }
        return $clean;
    }

    /** @return array<string, array<string, array<string, string>>> */
    private function texts(): array
    {
        /** @var array<string, array<string, array<string, string>>> */
        return $this->texts ??= $this->templates->data('mail/texts');
    }
}
