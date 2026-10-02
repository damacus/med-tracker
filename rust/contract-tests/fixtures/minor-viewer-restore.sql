SELECT 1 / ((COUNT(*) = 1)::integer)
FROM household_browser_minor_snapshot s
JOIN household_memberships m ON m.id = s.membership_id
JOIN accounts a ON a.id = s.account_id
WHERE s.membership_id = :'view_membership_id'::bigint
  AND s.household_id = :'household_id'::bigint
  AND s.account_id = :'view_account_id'::bigint
  AND m.household_id = s.household_id
  AND m.account_id = s.account_id
  AND m.person_id = s.person_id
  AND a.email = :'web_view_email';

WITH updated AS (
  UPDATE people p
  SET person_type = s.person_type,
      date_of_birth = s.date_of_birth,
      has_capacity = s.has_capacity
  FROM household_browser_minor_snapshot s
  WHERE p.id = s.person_id
    AND p.household_id = s.household_id
    AND p.account_id = s.account_id
  RETURNING p.id
)
SELECT 1 / ((COUNT(*) = 1)::integer) FROM updated;

SELECT 1 / ((COUNT(*) = 1)::integer)
FROM people p
JOIN household_browser_minor_snapshot s ON s.person_id = p.id
WHERE p.person_type IS NOT DISTINCT FROM s.person_type
  AND p.date_of_birth IS NOT DISTINCT FROM s.date_of_birth
  AND p.has_capacity IS NOT DISTINCT FROM s.has_capacity;

DROP TABLE household_browser_minor_snapshot;

