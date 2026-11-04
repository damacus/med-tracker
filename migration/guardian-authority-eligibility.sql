CREATE OR REPLACE FUNCTION med_tracker.sharing_guardian(p_subject bigint) RETURNS bigint
LANGUAGE plpgsql VOLATILE SECURITY DEFINER
SET search_path=pg_catalog SET plan_cache_mode=force_generic_plan SET join_collapse_limit=1 AS $body$
DECLARE previous_subject text; declaration bigint;
BEGIN
 previous_subject:=current_setting('med_tracker.sharing_subject_id',true);
 PERFORM set_config('med_tracker.sharing_subject_id',p_subject::text,true);
 SELECT d.id INTO declaration FROM public.guardian_declarations d
 JOIN public.accounts a ON a.person_id=d.guardian_person_id
 WHERE a.id=med_tracker.current_account_id() AND a.status=2 AND a.active
  AND NOT EXISTS(SELECT 1 FROM public.account_lockouts l WHERE l.account_id=a.id AND l.deadline>clock_timestamp())
  AND d.subject_person_id=p_subject AND d.withdrawn_at IS NULL AND d.superseded_at IS NULL
  AND med_tracker.sharing_declaration_eligible(d.id,p_subject)
 ORDER BY d.id LIMIT 1;
 PERFORM set_config('med_tracker.sharing_subject_id',COALESCE(previous_subject,''),true);
 RETURN declaration;
END $body$;
