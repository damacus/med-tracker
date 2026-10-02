CREATE TABLE household_browser_minor_snapshot AS
SELECT p.id AS person_id, p.person_type, p.date_of_birth, p.has_capacity,
       m.id AS membership_id, m.household_id, m.account_id
FROM people p
JOIN household_memberships m ON m.person_id = p.id
JOIN accounts a ON a.id = m.account_id
WHERE m.id = :'view_membership_id'::bigint
  AND m.household_id = :'household_id'::bigint
  AND m.account_id = :'view_account_id'::bigint
  AND p.household_id = m.household_id
  AND p.account_id = m.account_id
  AND a.email = :'web_view_email'
  AND m.role = 'member'
  AND m.status = 'active';

SELECT 1 / ((COUNT(*) = 1)::integer) FROM household_browser_minor_snapshot;

WITH updated AS (
  UPDATE people p
  SET person_type = 1,
      date_of_birth = (CURRENT_DATE - INTERVAL '10 years')::date,
      has_capacity = false
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
WHERE p.person_type = 1 AND p.has_capacity = false
  AND p.date_of_birth = (CURRENT_DATE - INTERVAL '10 years')::date;

