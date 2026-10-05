<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Embassy\EmbassyInbox;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The Embassy inbox sync for a cron job (Domain\Embassy\EmbassyInbox; the app's "Check mail" is POST
 * /api/embassy/inbox/sync): reads the new mails of EMBASSY_IMAP_FOLDER and logs the replies on their organisations.
 * Exit code 1 when the mailbox could not be read (the error is also stored and shown in the app).
 */
#[AsCommand(name: 'embassy:inbox', description: 'Read the Embassy mailbox (IMAP) and log the replies on their organisations')]
final class EmbassyInboxCommand extends Command
{
    public function __construct(private readonly EmbassyInbox $inbox)
    {
        parent::__construct();
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        try {
            $result = $this->inbox->run();
        } catch (ApiError $e) {
            $output->writeln('<error>' . $e->getMessage() . '</error>');
            return self::FAILURE;
        }
        if (!$result['configured']) {
            $output->writeln('Not configured (no EMBASSY_IMAP_HOST, and no Embassy mailbox login for SMTP_HOST) – nothing to do.');
            return self::SUCCESS;
        }
        $output->writeln(sprintf('Embassy inbox: %d matched, %d skipped, %d new activities.', $result['matched'] ?? 0, $result['skipped'] ?? 0, $result['new_activities'] ?? 0));
        if (($result['error'] ?? null) !== null) {
            $output->writeln('<error>' . $result['error'] . '</error>');
            return self::FAILURE;
        }
        return self::SUCCESS;
    }
}
