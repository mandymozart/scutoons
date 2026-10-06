-- Web search in the Ask panels (all apps): the Ask service lets the model search and read the web (Anthropic's server
-- tools web_search / web_fetch). A search is billed per request on top of the tokens, so the usage log counts them:
-- `agent_usage.web_searches` = web search requests of the call (web fetches cost only their tokens). `cost_usd` stays the
-- whole cost of the call (tokens + searches) – the admin dashboard's sums are the exact bill. Until it ran: the API
-- drops the count, the cost still includes the searches.
--
-- phpMyAdmin (production): select the database first (d0487143), SQL tab, run this file. Additive, MariaDB, idempotent
-- (IF NOT EXISTS); needs 2026-10-04-agent-usage.
ALTER TABLE `agent_usage` ADD COLUMN IF NOT EXISTS `web_searches` INT NOT NULL DEFAULT 0 COMMENT 'web search requests of the call (billed per search, included in cost_usd)';
