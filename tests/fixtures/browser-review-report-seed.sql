INSERT INTO public.person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,created_at,updated_at)
VALUES (78002,72001,74001,73002,'view','parent',now(),now());
INSERT INTO public.medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,created_at,updated_at)
SELECT 90000+number,'Synthetic reference','report-' || number,'https://example.test/evidence/' || number,'Synthetic tablets','Warnings',repeat('Representative clinical review evidence. ',20),date '2026-01-01','moderate','high',now(),now()
FROM generate_series(1,48) AS evidence(number);
INSERT INTO public.medication_review_prompts(id,household_id,person_id,primary_medication_id,interacting_medication_id,evidence_record_id,primary_medication_name,interacting_medication_name,evidence_source_name,evidence_source_url,evidence_source_version,evidence_source_checked_on,evidence_source_effective_on,evidence_text,match_confidence,match_reason,match_type,matched_term,risk_level,source_instruction,status,created_at,updated_at)
SELECT 91000+number,72001,73001,80001,80001,90000+number,'Synthetic tablets','Synthetic tablets','Synthetic reference','https://example.test/evidence/' || number,'1',date '2026-01-01',date '2026-01-01',repeat('Representative clinical review evidence. ',20),'high','Clinical review required for synthetic evidence ' || number,'ingredient','synthetic','moderate','unclassified','needs_review',now(),now()
FROM generate_series(1,48) AS prompt(number);
