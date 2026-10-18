CREATE OR REPLACE FUNCTION browser_revoke_grant_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE public.household_memberships SET status='revoked',revoked_at=timezone('UTC',clock_timestamp()),permissions_version=permissions_version+1 WHERE id=74001 AND household_id=72001 AND person_id=73001;
    RETURN NEW;
END $$;
CREATE TRIGGER browser_revoke_grant_on_prompt BEFORE INSERT ON public.medication_review_prompts
FOR EACH ROW EXECUTE FUNCTION browser_revoke_grant_on_prompt();
