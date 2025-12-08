-- Update default sort settings for instances to sort by creation date descending
UPDATE "AppConfiguration"
SET "instancesSortBy" = 'created',
    "instancesSortByAsc" = false
WHERE "instancesSortBy" = 'name' AND "instancesSortByAsc" = true;
