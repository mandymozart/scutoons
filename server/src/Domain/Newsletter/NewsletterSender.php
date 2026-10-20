<?php
declare(strict_types=1);

namespace Scutoons\Domain\Newsletter;

use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Mail\MailLog;
use Scutoons\Domain\Settings\RuntimeSettings;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Mail\SmtpFailure;
use Scutoons\Infrastructure\Mail\Transport;
use Throwable;

/**
 * Sends the newsletters in small batches so the SMTP account is never flooded (agents/NEWSLETTER.md "Sending"). One
 * batch = one call of batch(): from the host's cron (GET|POST /api/newsletter/cron?key=…, every few minutes) or "Send
 * now" in the admin (the page calls it again while it is open). A batch
 *   - runs alone (a MariaDB named lock – a second call at the same time answers busy)
 *   - sends at most the runtime setting newsletter_batch (default NEWSLETTER_BATCH) mails and stops after NEWSLETTER_SECONDS
 *   - never goes over the runtime setting newsletter_per_hour: every mail sent in the last hour counts (mail_log, the
 *     login codes too, so they always get through)
 *   - keeps ONE SMTP connection (Transport::beginBatch), a new one every NEWSLETTER_PER_CONNECTION mails, waits
 *     NEWSLETTER_DELAY_MS between two mails
 *   - per recipient: sent; refused for good (5xx at RCPT) → failed + suppressed (bounce); any other failure → tried
 *     again after NEWSLETTER_RETRY_MINUTES, failed after NEWSLETTER_ATTEMPTS; the SMTP login / connection fails → the
 *     batch stops, nobody's attempt counts, the newsletter shows the error (last_error)
 *   - skips an address suppressed since the start (unsubscribed meanwhile)
 *   - marks a newsletter done when nobody is queued any more
 * Every mail is logged in mail_log as template `newsletter`. Mails carry List-Unsubscribe + List-Unsubscribe-Post
 * (RFC 8058 one-click: the mail client's own "Unsubscribe" button).
 *
 * @phpstan-type BatchResult array{status: string, sent: int, failed: int, retry: int, skipped: int, queued: int, per_hour: int, sent_last_hour: int, error: ?string}
 */
final class NewsletterSender
{
    public const TEMPLATE = 'newsletter';
    private const LOCK = 'scutoons_newsletter_batch';

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly RuntimeSettings $runtime,
        private readonly Transport $transport,
        private readonly NewsletterRenderer $renderer,
        private readonly NewsletterSignin $signin,
        private readonly Giveaways $giveaways,
        private readonly Suppressions $suppressions,
        private readonly MailLog $log,
    ) {
    }

    /**
     * One batch.
     * @return BatchResult status: idle (nothing to send) | sent (a batch went out; queued > 0 = more to come) | limit (the hourly limit is
     *         reached) | busy (another batch is running) | error (SMTP login / connection – see error)
     */
    public function batch(): array
    {
        $result = ['status' => 'idle', 'sent' => 0, 'failed' => 0, 'retry' => 0, 'skipped' => 0, 'queued' => 0,
            'per_hour' => $this->runtime->newsletterPerHour(), 'sent_last_hour' => 0, 'error' => null];
        if ((int) $this->db->value('SELECT GET_LOCK(?, 0)', [self::LOCK]) !== 1) {
            $result['status'] = 'busy';
            return $result;
        }
        try {
            $this->run($result);
        } finally {
            $this->transport->endBatch();
            $this->db->value('SELECT RELEASE_LOCK(?)', [self::LOCK]);
        }
        $result['queued'] = $this->queued();
        $result['sent_last_hour'] = $this->sentLastHour();
        return $result;
    }

    /**
     * @param BatchResult $result
     * @param-out BatchResult $result
     */
    private function run(array &$result): void
    {
        $deadline = microtime(true) + max(1, $this->settings->int('NEWSLETTER_SECONDS'));
        $perHour = $result['per_hour'];
        $budget = $this->runtime->newsletterBatch();
        if ($perHour > 0) $budget = min($budget, $perHour - $this->sentLastHour());
        $perConnection = max(1, $this->settings->int('NEWSLETTER_PER_CONNECTION'));
        $delay = max(0, $this->settings->int('NEWSLETTER_DELAY_MS'));
        $first = true;

        $campaigns = $this->db->all("SELECT * FROM mail_campaigns WHERE status = 'sending' ORDER BY started_at, id");
        foreach ($campaigns as $campaign) {
            $id = (int) $campaign['id'];
            $due = $this->db->all("SELECT * FROM mail_campaign_recipients WHERE campaign_id = ? AND status = 'queued'
                AND (next_attempt_at IS NULL OR next_attempt_at <= ?) ORDER BY id LIMIT " . max(1, (int) $budget), [$id, $this->clock->utc()]);
            if ($due !== [] && $budget <= 0) {
                if ($result['status'] === 'idle') $result['status'] = 'limit'; // nothing sent: the hourly limit
                return;
            }
            if ($due !== [] && $result['status'] === 'idle') $this->transport->beginBatch();
            foreach ($due as $recipient) {
                if ($budget <= 0 || microtime(true) >= $deadline) break 2;
                if ($this->db->value('SELECT status FROM mail_campaigns WHERE id = ?', [$id]) !== 'sending') break; // paused meanwhile
                $email = (string) $recipient['email'];
                if ($this->suppressions->has($email)) {
                    $this->finish((int) $recipient['id'], 'skipped', 'suppressed after the start');
                    $result['skipped']++;
                    continue;
                }
                if (!$first && $delay > 0) usleep($delay * 1000);
                $first = false;
                if ($this->transport->batchDelivered() >= $perConnection) $this->transport->reconnect();
                $result['status'] = 'sent';
                $budget--;
                $locale = (string) $recipient['locale'];
                try {
                    $mail = $this->renderer->render(
                        ['subject' => (string) $campaign["subject_$locale"], 'body' => (string) $campaign["body_$locale"]],
                        $locale, $email, $recipient['name'] !== null ? (string) $recipient['name'] : null,
                        fn(string $kind) => $kind === 'gift' ? $this->giveaways->buttonFor($campaign, $email, $locale) : $this->signin->button($email, $locale));
                    $this->transport->deliverWithHeaders($email, $mail['subject'], $mail['text'], $mail['html'], [
                        'List-Unsubscribe: <' . $mail['unsubscribe'] . '>',
                        'List-Unsubscribe-Post: List-Unsubscribe=One-Click',
                    ]);
                } catch (Throwable $error) {
                    $message = mb_substr($error->getMessage(), 0, 500);
                    if ($error instanceof SmtpFailure && $error->aboutTheServer()) {
                        // the account / the server: every further mail would fail too – stop, count nobody's attempt
                        $this->db->run('UPDATE mail_campaigns SET last_error = ? WHERE id = ?', [$message, $id]);
                        $result['status'] = 'error';
                        $result['error'] = $message;
                        return;
                    }
                    $this->log->record(self::TEMPLATE, $email, $locale, 'failed', $this->transport->name(), $message);
                    if ($error instanceof SmtpFailure && $error->permanentForRecipient()) {
                        $this->finish((int) $recipient['id'], 'failed', $message);
                        $this->suppressions->add($email, 'bounce', $message);
                        $result['failed']++;
                    } elseif ((int) $recipient['attempts'] + 1 >= max(1, $this->settings->int('NEWSLETTER_ATTEMPTS'))) {
                        $this->finish((int) $recipient['id'], 'failed', $message);
                        $result['failed']++;
                    } else {
                        $this->db->run('UPDATE mail_campaign_recipients SET attempts = attempts + 1, next_attempt_at = ?, error = ? WHERE id = ?',
                            [$this->clock->utc(60 * max(1, $this->settings->int('NEWSLETTER_RETRY_MINUTES'))), $message, (int) $recipient['id']]);
                        $result['retry']++;
                    }
                    continue;
                }
                $this->db->run("UPDATE mail_campaign_recipients SET status = 'sent', attempts = attempts + 1, sent_at = ?, error = NULL WHERE id = ?",
                    [$this->clock->utc(), (int) $recipient['id']]);
                $this->log->record(self::TEMPLATE, $email, $locale, 'sent', $this->transport->name(), null);
                $result['sent']++;
            }
            $this->db->run('UPDATE mail_campaigns SET last_error = NULL WHERE id = ? AND last_error IS NOT NULL', [$id]);
            $this->finishCampaign($id);
        }
    }

    private function finish(int $recipientId, string $status, string $error): void
    {
        $this->db->run('UPDATE mail_campaign_recipients SET status = ?, attempts = attempts + 1, error = ? WHERE id = ?', [$status, mb_substr($error, 0, 500), $recipientId]);
    }

    /** sending → done once nobody is queued */
    private function finishCampaign(int $id): void
    {
        if ($this->db->value("SELECT 1 FROM mail_campaign_recipients WHERE campaign_id = ? AND status = 'queued' LIMIT 1", [$id]) !== null) return;
        $now = $this->clock->utc();
        $this->db->run("UPDATE mail_campaigns SET status = 'done', finished_at = ?, updated_at = ? WHERE id = ? AND status = 'sending'", [$now, $now, $id]);
    }

    /** Recipients still queued in newsletters that are sending */
    private function queued(): int
    {
        return (int) $this->db->value("SELECT COUNT(*) FROM mail_campaign_recipients r JOIN mail_campaigns c ON c.id = r.campaign_id
            WHERE c.status = 'sending' AND r.status = 'queued'");
    }

    /** Every mail the API sent in the last hour (mail_log – the login codes too) */
    private function sentLastHour(): int
    {
        return (int) $this->db->value("SELECT COUNT(*) FROM mail_log WHERE status = 'sent' AND created_at >= ?", [$this->clock->utc(-3600)]);
    }
}
