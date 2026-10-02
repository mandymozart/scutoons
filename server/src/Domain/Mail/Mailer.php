<?php
declare(strict_types=1);

namespace Scutoons\Domain\Mail;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Mail\Transport;
use Throwable;

/**
 * THE way the platform sends email (agents/PLATFORM.md "Emails"): a registered template (MailTemplates) is rendered
 * in the recipient's locale (MailRenderer), handed to the transport (Infrastructure\Mail\Transport: log | mail | smtp)
 * and recorded in the sent-mail log (MailLog, admin page "Emails").
 *
 *   $mailer->send(MailTemplates::LOGIN_CODE, $email, $locale, ['link' => …, 'otp' => …, 'minutes' => 30]);
 *   $mailer->sendToAdmins(MailTemplates::SOME_ADMIN_TEMPLATE, [...]);   // MailRecipients::admins()
 */
final class Mailer
{
    public function __construct(
        private readonly MailRenderer $renderer,
        private readonly Transport $transport,
        private readonly MailLog $log,
        private readonly MailRecipients $recipients,
        private readonly Settings $settings,
    ) {
    }

    /**
     * Renders and sends one mail; logs it as `sent`, or as `failed` with the error and throws MailFailed.
     * $locale: de | en (anything else, e.g. "de-AT" → de, unknown → en).
     * @param array<string, mixed> $vars the template's variables (MailTemplates "vars")
     * @return int the mail log id (0 = not logged)
     * @throws MailFailed the transport refused it
     * @throws \InvalidArgumentException unknown template / missing variable (a bug, not logged)
     */
    public function send(string $template, string $to, string $locale, array $vars = []): int
    {
        $locale = MailTemplates::locale($locale);
        $mail = $this->renderer->render($template, $locale, $vars);
        $transport = $this->transport->name();
        try {
            $this->transport->deliver($to, $mail['subject'], $mail['text'], $mail['html']);
        } catch (Throwable $error) {
            $id = $this->log->record($template, $to, $locale, 'failed', $transport, $error->getMessage());
            error_log("[qr-api] mail $template failed: " . $error->getMessage());
            throw new MailFailed($template, $id, "The $template mail could not be sent: " . $error->getMessage(), $error);
        }
        return $this->log->record($template, $to, $locale, 'sent', $transport, null);
    }

    /**
     * Sends a mail to every admin recipient (MailRecipients: the Emails page, default MAIL_ADMIN_TO) in
     * MAIL_ADMIN_LOCALE (default de). A failure for one address is logged and does not stop the others.
     * @param array<string, mixed> $vars
     * @return array{sent: int, failed: int} nothing configured → both 0 (logged to the PHP error log)
     */
    public function sendToAdmins(string $template, array $vars = []): array
    {
        $result = ['sent' => 0, 'failed' => 0];
        $to = $this->recipients->admins();
        if ($to === []) {
            error_log("[qr-api] mail $template: no admin recipients (admin app → Emails, or MAIL_ADMIN_TO)");
            return $result;
        }
        foreach ($to as $address) {
            try {
                $this->send($template, $address, $this->settings->get('MAIL_ADMIN_LOCALE'), $vars);
                $result['sent']++;
            } catch (MailFailed) {
                $result['failed']++;
            }
        }
        return $result;
    }
}
