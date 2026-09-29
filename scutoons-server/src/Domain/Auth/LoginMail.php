<?php
declare(strict_types=1);

namespace Scutoons\Domain\Auth;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * The login email: a magic link and a 6-digit one-time code (typed into the page/app – needed where the link opens
 * another browser). Mentions the code that will be added to the inventory.
 * composeNoAccount(): the mail for an address without an account that asked without a signup code – no link, no
 * one-time code, just how to get an account. Texts: templates/mail/texts.php; HTML: templates/mail/*.html.php.
 */
final class LoginMail
{
    /** @var array<string, array<string, string>>|null */
    private ?array $texts = null;

    public function __construct(private readonly Templates $templates, private readonly Settings $settings)
    {
    }

    /** de or en (default) from a locale like "de-AT" */
    public function locale(mixed $value): string
    {
        if (!is_string($value)) return 'en';
        $locale = strtolower(substr($value, 0, 2));
        return isset($this->texts()[$locale]) ? $locale : 'en';
    }

    /**
     * $claimCode = a claimable code that is added to the inventory; $signup = confirming creates the account
     * @return array{subject: string, text: string, html: string}
     */
    public function compose(string $locale, string $link, string $otp, ?string $claimCode, bool $signup = false): array
    {
        $t = $this->translator($locale, ['{otp}' => $otp, '{code}' => (string) $claimCode, '{minutes}' => (string) $this->settings->int('LOGIN_TTL_MINUTES')]);

        $text = $t('intro') . "\n\n";
        if ($signup) $text .= $t('signup') . "\n\n";
        if ($claimCode !== null) $text .= $t('claim') . "\n\n";
        $text .= $link . "\n\n" . $t('otp') . "\n\n    " . $otp . "\n\n" . $t('validity') . "\n" . $t('ignore') . "\n\n– SCUTOONS\n";

        $html = $this->templates->render('mail/login.html', compact('locale', 'link', 'otp', 'signup', 'claimCode', 't'));
        return ['subject' => $t('subject'), 'text' => $text, 'html' => $html];
    }

    /**
     * No account for this address and no usable signup code: explains how to get an account. $code = the code sent
     * with the request (not usable for a signup), if any.
     * @return array{subject: string, text: string, html: string}
     */
    public function composeNoAccount(string $locale, string $viewerUrl, ?string $code): array
    {
        $t = $this->translator($locale, ['{code}' => (string) $code]);
        $paragraphs = array_values(array_filter([$t('na_intro'), $code !== null ? $t('na_code') : null, $t('na_how'), $t('na_read')]));

        $text = implode("\n\n", $paragraphs) . "\n\n" . $viewerUrl . "\n\n" . $t('na_ignore') . "\n\n– SCUTOONS\n";
        $html = $this->templates->render('mail/no-account.html', compact('locale', 'viewerUrl', 'paragraphs', 't'));
        return ['subject' => $t('na_subject'), 'text' => $text, 'html' => $html];
    }

    /**
     * @param array<string, string> $vars
     * @return callable(string): string
     */
    private function translator(string $locale, array $vars): callable
    {
        $texts = $this->texts()[$locale] ?? $this->texts()['en'];
        return static fn(string $key): string => strtr($texts[$key], $vars);
    }

    /** @return array<string, array<string, string>> */
    private function texts(): array
    {
        /** @var array<string, array<string, string>> */
        return $this->texts ??= $this->templates->data('mail/texts');
    }
}
