-- Greenfield no-op migration.
-- Statement-aware schema and duplicate metadata are now defined directly in bootstrap migrations.
-- This migration intentionally exists to advance PRAGMA user_version to 3.
SELECT 1;
