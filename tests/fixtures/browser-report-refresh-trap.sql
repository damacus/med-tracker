INSERT INTO public.medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at)
VALUES (80011,72001,79001,'Synthetic capsules',10,1,'capsule',now(),now());
INSERT INTO public.person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at)
VALUES (81011,72001,73001,80011,1,'capsule',1,now(),now());
INSERT INTO public.medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at)
VALUES (90060,'Synthetic reference','report-60','https://example.test/evidence/60','Synthetic capsules','Warnings','Synthetic capsules increase exposure to synthetic compounds.',date '2026-01-01','moderate','high','reviewed_pair','{synthetic tablets}','{synthetic capsules}',now(),now());
