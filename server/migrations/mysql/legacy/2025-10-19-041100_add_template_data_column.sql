-- Add template_data column to qr table
-- This stores the template field data (receiver, productname, edition_number, etc.)

ALTER TABLE `qr` 
ADD COLUMN `template_data` longtext CHARACTER SET utf8mb4 COLLATE utf8mb4_bin DEFAULT NULL 
CHECK (json_valid(`template_data`))
AFTER `signature`;
