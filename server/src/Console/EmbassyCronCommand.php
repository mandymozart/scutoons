<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Embassy\EmbassyReminders;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The Embassy's daily job, like the cron URL GET /api/embassy/cron?key=… (Domain\Embassy\EmbassyReminders): the inbox
 * sync, then the due reminder mails to the team members (weekly / daily as each chose). For local testing and hosts
 * whose cron can run PHP. Exit code 1 when the mailbox could not be read.
 */
#[AsCommand(name: 'embassy:cron', description: 'Embassy daily job: inbox sync + the reminder mails to the team (weekly / daily)')]
final class EmbassyCronCommand extends Command
{
    public function __construct(private readonly EmbassyReminders $reminders)
    {
        parent::__construct();
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $result = $this->reminders->cron();
        $inbox = $result['inbox'];
        $output->writeln($inbox === null ? 'Inbox: not configured (no EMBASSY_IMAP_HOST, and no Embassy mailbox login for SMTP_HOST).'
            : sprintf('Inbox: %d matched, %d skipped, %d new activities.', $inbox['matched'] ?? 0, $inbox['skipped'] ?? 0, $inbox['new_activities'] ?? 0));
        $reminders = $result['reminders'];
        $output->writeln($reminders === null ? 'Reminders: not available (migration 2026-10-05-embassy-pace).'
            : sprintf('Reminders: %d sent, %d skipped, %d failed.', $reminders['sent'], $reminders['skipped'], $reminders['failed']));
        if (($inbox['error'] ?? null) !== null) {
            $output->writeln('<error>' . $inbox['error'] . '</error>');
            return self::FAILURE;
        }
        return self::SUCCESS;
    }
}
