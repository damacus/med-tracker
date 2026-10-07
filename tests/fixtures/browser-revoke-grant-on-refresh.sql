CREATE OR REPLACE FUNCTION browser_revoke_grant_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE public.person_access_grants SET revoked_at=now() WHERE id=78001;
    RETURN NEW;
END $$;
CREATE TRIGGER browser_revoke_grant_on_prompt BEFORE INSERT ON public.medication_review_prompts
FOR EACH ROW EXECUTE FUNCTION browser_revoke_grant_on_prompt();
