-- The Ask panel's tables go (agents/MCP.md, owner 2026-10-09: Ask is gone – members use their own Claude through the
-- SCUTOONS MCP; no export of the chats / the usage log, D5). Run ONLY once the move is done (owner: "we do this once the
-- move is done") – the code no longer reads them since commit 560104a1. MariaDB 10.11, idempotent – phpMyAdmin
-- (database d0487143 selected).
--
-- Handed to the owner for production on 2026-10-09 (the move is done); in migrations/mysql/ since, so the dev databases follow.
--
-- Look first (what goes):
--   SELECT 'agent_chats' AS t, COUNT(*) FROM agent_chats UNION ALL SELECT 'agent_usage', COUNT(*) FROM agent_usage;

DROP TABLE IF EXISTS `agent_chats`;
DROP TABLE IF EXISTS `agent_usage`;
