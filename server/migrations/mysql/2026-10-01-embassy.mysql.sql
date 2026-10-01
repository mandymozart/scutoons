-- Embassy (embassy.scutoons.com): the partner network of Top Toy / SCUTOONS – organisations (venues, festivals,
-- radios, shops, agents, press …), their people, one lead per organisation × track (live, vinyl, booking, promo) with
-- stage / owner / next check date, and the activity timeline (mails, calls, notes). Roles liaison (part time) and
-- ambassador (full time). Domain\Embassy\Embassy, API /api/embassy/*, agents/EMBASSY.md. MariaDB, idempotent.
INSERT IGNORE INTO `roles` (`id`, `name`, `description`) VALUES
  (4, 'liaison', 'Embassy (part time): works the leads, calls, mails, notes'),
  (5, 'ambassador', 'Embassy (full time): everything a liaison does, plus assigning, importing, deleting');

CREATE TABLE IF NOT EXISTS `embassy_orgs` (
  `id` CHAR(36) NOT NULL,
  `name` VARCHAR(255) NOT NULL,
  `type` VARCHAR(32) NOT NULL DEFAULT 'other',
  `city` VARCHAR(255) NOT NULL DEFAULT '',
  `country` VARCHAR(2) NOT NULL DEFAULT '' COMMENT 'ISO 3166-1 alpha-2, upper case',
  `website` VARCHAR(1024) NOT NULL DEFAULT '',
  `email` VARCHAR(255) NOT NULL DEFAULT '',
  `phone` VARCHAR(64) NOT NULL DEFAULT '',
  `links` JSON NOT NULL COMMENT 'list of URLs (Instagram, forms, …)',
  `tags` JSON NOT NULL COMMENT 'list of free tags (genres, scene)',
  `territories` JSON NOT NULL COMMENT 'list of ISO country codes they cover (agents)',
  `tier` CHAR(1) NULL DEFAULT NULL COMMENT 'A | B | C',
  `source` VARCHAR(255) NOT NULL DEFAULT '',
  `notes` TEXT NOT NULL,
  `do_not_contact` TINYINT(1) NOT NULL DEFAULT 0,
  `name_key` VARCHAR(255) NOT NULL COMMENT 'normalized name for duplicate checks',
  `created_at` DATETIME NOT NULL COMMENT 'UTC',
  `updated_at` DATETIME NOT NULL COMMENT 'UTC',
  `created_by` VARCHAR(255) NULL DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `embassy_orgs_name_key` (`name_key`),
  KEY `embassy_orgs_country` (`country`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `embassy_people` (
  `id` CHAR(36) NOT NULL,
  `org_id` CHAR(36) NOT NULL,
  `name` VARCHAR(255) NOT NULL DEFAULT '',
  `role` VARCHAR(255) NOT NULL DEFAULT '',
  `email` VARCHAR(255) NOT NULL DEFAULT '',
  `phone` VARCHAR(64) NOT NULL DEFAULT '',
  `link` VARCHAR(1024) NOT NULL DEFAULT '',
  `language` VARCHAR(16) NOT NULL DEFAULT '',
  `territories` JSON NOT NULL,
  `notes` TEXT NOT NULL,
  `do_not_contact` TINYINT(1) NOT NULL DEFAULT 0,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  KEY `embassy_people_org` (`org_id`),
  CONSTRAINT `embassy_people_org_fk` FOREIGN KEY (`org_id`) REFERENCES `embassy_orgs` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `embassy_leads` (
  `id` CHAR(36) NOT NULL,
  `org_id` CHAR(36) NOT NULL,
  `track` VARCHAR(16) NOT NULL COMMENT 'live | vinyl | booking | promo',
  `stage` VARCHAR(32) NOT NULL DEFAULT 'research',
  `owner_id` CHAR(36) NULL DEFAULT NULL,
  `next_at` DATE NULL DEFAULT NULL COMMENT 'next check date (local)',
  `last_contact_at` DATETIME NULL DEFAULT NULL,
  `created_at` DATETIME NOT NULL,
  `updated_at` DATETIME NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `embassy_leads_org_track` (`org_id`, `track`),
  KEY `embassy_leads_next` (`next_at`),
  KEY `embassy_leads_owner` (`owner_id`),
  CONSTRAINT `embassy_leads_org_fk` FOREIGN KEY (`org_id`) REFERENCES `embassy_orgs` (`id`) ON DELETE CASCADE,
  CONSTRAINT `embassy_leads_owner_fk` FOREIGN KEY (`owner_id`) REFERENCES `users` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `embassy_activities` (
  `id` CHAR(36) NOT NULL,
  `org_id` CHAR(36) NOT NULL,
  `lead_id` CHAR(36) NULL DEFAULT NULL,
  `person_id` CHAR(36) NULL DEFAULT NULL,
  `kind` VARCHAR(16) NOT NULL COMMENT 'mail_out | mail_in | call | dm | meeting | note',
  `outcome` VARCHAR(16) NULL DEFAULT NULL COMMENT 'calls: reached | voicemail | no_answer | call_back | declined',
  `note` TEXT NOT NULL,
  `by_user_id` CHAR(36) NULL DEFAULT NULL,
  `by_email` VARCHAR(255) NOT NULL DEFAULT '',
  `at` DATETIME NOT NULL COMMENT 'UTC',
  PRIMARY KEY (`id`),
  KEY `embassy_activities_org` (`org_id`, `at`),
  CONSTRAINT `embassy_activities_org_fk` FOREIGN KEY (`org_id`) REFERENCES `embassy_orgs` (`id`) ON DELETE CASCADE,
  CONSTRAINT `embassy_activities_lead_fk` FOREIGN KEY (`lead_id`) REFERENCES `embassy_leads` (`id`) ON DELETE SET NULL,
  CONSTRAINT `embassy_activities_person_fk` FOREIGN KEY (`person_id`) REFERENCES `embassy_people` (`id`) ON DELETE SET NULL,
  CONSTRAINT `embassy_activities_user_fk` FOREIGN KEY (`by_user_id`) REFERENCES `users` (`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
