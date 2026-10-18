<?php
declare(strict_types=1);

namespace Scutoons\Domain\Embassy;

use DateTimeImmutable;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Push\TeamPush;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Templates\Templates;

/**
 * The Embassy's daily job (owner 2026-10-05; GET|POST /api/embassy/cron?key=… from the host's cron, or
 * `bin/console embassy:cron`): (a) the inbox sync (EmbassyInbox), (b) the reminder mails – each team member's own
 * portion (EmbassyPace), as they chose: `weekly` = on Monday (or the first run of the week when Monday's was missed),
 * `daily` = Mon–Fri, `off` = never.
 *
 * Rules: at most one mail per local day (daily) / per week (weekly) – `reminded_at`; only people who still have an
 * Embassy role; no mail when the portion has no items and no check-in is due. The mail is the registered template
 * `embassy_portion` (MailTemplates, from MAIL_FROM, in the account's language, a mail_log row like every mail) with a
 * link to the Today page (EMBASSY_URL). Reminders go ONLY to the team member – nothing here ever mails a contact.
 * Channels (owner 2026-10-18, research/PWA-APPS.md §5): the same rhythm by mail and / or push – the person's switches
 * `embassy.mail` and `embassy.due` (TeamPush, both on by default; the push only reaches a browser that switched it on).
 */
final class EmbassyReminders
{
    /** @var array<string, array<string, string>>|null locale → texts of embassy_portion */
    private ?array $texts = null;

    public function __construct(
        private readonly Embassy $embassy,
        private readonly EmbassyPace $pace,
        private readonly EmbassyInbox $inbox,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
        private readonly Templates $templates,
        private readonly TeamPush $push,
    ) {
    }

    /**
     * The cron run: `inbox` = EmbassyInbox::run() (null: not configured), `reminders` (null: migration
     * 2026-10-05-embassy-pace not run)
     * @return array{inbox: array<string, mixed>|null, reminders: array{sent: int, skipped: int, failed: int}|null}
     */
    public function cron(): array
    {
        $inbox = null;
        if ($this->inbox->configured()) {
            try {
                $inbox = $this->inbox->run();
            } catch (ApiError $error) {
                $inbox = ['configured' => true, 'error' => $error->getMessage()];
            }
        }
        return ['inbox' => $inbox, 'reminders' => $this->pace->available() ? $this->send() : null];
    }

    /**
     * Sends the reminders that are due now. `skipped` = reminder on but not due (not their day, sent already) or
     * nothing to say; `failed` = the transport refused it (tried again on the next run).
     * @return array{sent: int, skipped: int, failed: int}
     */
    public function send(): array
    {
        $result = ['sent' => 0, 'skipped' => 0, 'failed' => 0];
        $today = $this->embassy->todayDate();
        $weekday = (int) (new DateTimeImmutable($today))->format('N');
        [$monday] = EmbassyPace::week($today);
        $team = [];
        foreach ($this->embassy->team() as $member) $team[$member['id']] = true;
        foreach ($this->pace->reminderRows() as $row) {
            if (!isset($team[$row['user_id']])) continue; // the role was taken away
            $last = $row['reminded_at'] !== null ? EmbassyPace::localDate($row['reminded_at']) : null;
            $due = $row['reminder'] === 'daily' ? $weekday <= 5 && $last !== $today : $last === null || $last < $monday;
            $portion = $due ? $this->pace->portionOf($row['user_id']) : null;
            $checkin = (bool) ($this->pace->of($row['user_id'])['checkin_due'] ?? false);
            if ($portion === null || ($portion['items'] === [] && !$checkin)) {
                $result['skipped']++;
                continue;
            }
            $locale = MailTemplates::locale($row['locale']);
            $daily = $row['reminder'] === 'daily';
            $mailed = false;
            $mailFailed = false;
            if ($this->channel($row['user_id'], 'embassy.mail')) {
                try {
                    $this->mailer->send(MailTemplates::EMBASSY_PORTION, $row['email'], $locale, $this->vars($portion, $daily, $checkin, $locale));
                    $mailed = true;
                } catch (MailFailed) {
                    $mailFailed = true;
                }
            }
            // the push says how many are due (today's slice when daily) – a check-in alone is the mail's
            $count = $daily ? min(count($portion['items']), (int) $portion['today_count']) : count($portion['items']);
            $pushed = false;
            try {
                $pushed = $this->push->embassyDue($row['user_id'], $count, $daily, $today);
            } catch (\Throwable) {
                // push trouble: the mail decides
            }
            if (!$mailed && !$pushed) {
                $result[$mailFailed ? 'failed' : 'skipped']++;
                continue;
            }
            $this->pace->markReminded($row['user_id']);
            $result['sent']++;
        }
        return $result;
    }

    /** the person's switch for a channel (on by default; before the push migration: the mail as before) */
    private function channel(string $userId, string $kind): bool
    {
        try {
            return $this->push->on($userId, $kind);
        } catch (\Throwable) {
            return true;
        }
    }

    /**
     * The template's variables: the week's items (weekly) or today's slice (daily), "+N more" for the rest
     * @param array<string, mixed> $portion EmbassyPace::portionOf()
     * @return array<string, mixed>
     */
    public function vars(array $portion, bool $daily, bool $checkin, string $locale): array
    {
        /** @var list<array<string, mixed>> $items */
        $items = $portion['items'];
        $shown = $daily ? array_slice($items, 0, (int) $portion['today_count']) : $items;
        $more = count($items) - count($shown) + array_sum((array) $portion['more']);
        $lines = array_map(fn(array $item) => $this->line($item, $locale), $shown);
        return [
            'done' => (int) $portion['week']['done'], 'pace' => (int) $portion['week']['pace'],
            'items' => $lines !== [] ? implode("\n", $lines) : null, 'more' => $more > 0 ? (string) $more : null,
            'daily' => $daily && $lines !== [], 'weekly' => !$daily && $lines !== [], 'checkin' => $checkin,
            'url' => rtrim($this->settings->get('EMBASSY_URL'), '/') . '/',
        ];
    }

    /**
     * "• Rote Fabrik · Lea – mail 6 days ago, no answer yet (“Top Toy”)"
     * @param array<string, mixed> $item
     */
    public function line(array $item, string $locale): string
    {
        $person = $item['people'][0]['name'] ?? '';
        return '• ' . $item['org']['name'] . ($person !== '' ? ' · ' . $person : '') . ' – ' . $this->hintText($item['hint'], $locale);
    }

    /**
     * A hint (EmbassyPace::HINTS) in words – texts in templates/mail/texts.php (embassy_portion.hint_*)
     * @param array<string, mixed> $hint
     */
    public function hintText(array $hint, string $locale): string
    {
        $t = $this->texts(MailTemplates::locale($locale));
        $days = (int) ($hint['days'] ?? 0);
        $ago = strtr($t['ago_' . ($days <= 1 ? $days : 'n')], ['{days}' => (string) $days]);
        $date = isset($hint['date']) ? (new DateTimeImmutable((string) $hint['date']))->format($t['date_format']) : '';
        $kind = (string) ($hint['kind'] ?? 'due');
        $text = strtr($t['hint_' . $kind] ?? $t['hint_due'], ['{ago}' => $ago, '{date}' => $date]);
        if (isset($hint['subject']) && $hint['subject'] !== '') $text .= strtr($t['hint_subject'], ['{subject}' => (string) $hint['subject']]);
        return $text;
    }

    /** @return array<string, string> */
    private function texts(string $locale): array
    {
        if ($this->texts === null) {
            $all = $this->templates->data('mail/texts');
            $this->texts = [];
            foreach (MailTemplates::LOCALES as $code) {
                /** @var array<string, string> $texts */
                $texts = $all[$code][MailTemplates::EMBASSY_PORTION] ?? [];
                $this->texts[$code] = $texts;
            }
        }
        return $this->texts[$locale] ?? $this->texts[MailTemplates::DEFAULT_LOCALE];
    }
}
