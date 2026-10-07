CREATE OR REPLACE FUNCTION browser_revoke_session_on_prompt() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM public.identity_sessions WHERE account_id=71001;
    RETURN NEW;
END $$;
CREATE TRIGGER browser_revoke_session_on_prompt BEFORE INSERT ON public.medication_review_prompts
FOR EACH ROW EXECUTE FUNCTION browser_revoke_session_on_prompt();
