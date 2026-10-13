UPDATE public.medication_review_prompts p
SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text)
FROM public.medication_review_evidence_records e
WHERE p.id=91001 AND p.evidence_record_id=e.id;
UPDATE public.medication_review_evidence_records
SET evidence_text = 'Changed synthetic evidence while the edit lease remains live.',
    source_version = '2',
    updated_at = clock_timestamp()
WHERE id = 90001;
