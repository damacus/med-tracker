ALTER TABLE public.api_app_tokens ADD CONSTRAINT api_app_token_positive_lifetime CHECK ((expires_at > created_at));

ALTER TABLE public.audit_chain_heads ADD CONSTRAINT audit_chain_heads_last_hash_length CHECK (((last_hash IS NULL) OR (octet_length(last_hash) = 32)));

ALTER TABLE public.audit_checkpoints ADD CONSTRAINT audit_checkpoint_entry_hash_length CHECK ((octet_length(entry_hash) = 32));

ALTER TABLE public.audit_export_deliveries ADD CONSTRAINT audit_export_delivery_exactly_one_record CHECK (((audit_ledger_entry_id IS NULL) <> (audit_checkpoint_id IS NULL)));

ALTER TABLE public.audit_ledger_entries ADD CONSTRAINT audit_ledger_entry_hash_length CHECK ((octet_length(entry_hash) = 32));

ALTER TABLE public.audit_ledger_entries ADD CONSTRAINT audit_ledger_positive_sequence CHECK ((sequence > 0));

ALTER TABLE public.audit_ledger_entries ADD CONSTRAINT audit_ledger_previous_hash_length CHECK (((previous_hash IS NULL) OR (octet_length(previous_hash) = 32)));

ALTER TABLE public.audit_ledger_entries ADD CONSTRAINT audit_ledger_retention_after_event CHECK ((retain_until >= occurred_at));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_exact_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_note CHECK (((note IS NULL) OR (char_length(note) <= 2000)));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_position CHECK (("position" > 0));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_reason CHECK (((reason IS NULL) OR ((reason)::text = ANY ((ARRAY['refused'::character varying, 'unwell'::character varying, 'asleep'::character varying, 'medicine_unavailable'::character varying, 'clinician_advice'::character varying, 'other'::character varying])::text[]))));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_state CHECK (((((outcome)::text = 'open'::text) AND (medication_take_id IS NULL) AND (reason IS NULL) AND (note IS NULL) AND (resolved_at IS NULL) AND (resolved_by_membership_id IS NULL)) OR (((outcome)::text = 'not_taken'::text) AND (medication_take_id IS NULL) AND (resolved_at IS NOT NULL) AND (resolved_by_membership_id IS NOT NULL)) OR (((outcome)::text = 'taken'::text) AND (medication_take_id IS NOT NULL) AND (reason IS NULL) AND (note IS NULL) AND (resolved_at IS NOT NULL) AND (resolved_by_membership_id IS NOT NULL))));

ALTER TABLE public.medication_dose_occurrences ADD CONSTRAINT chk_dose_occurrences_window_order CHECK ((window_ends_on >= window_starts_on));

ALTER TABLE public.medication_pause_periods ADD CONSTRAINT chk_medication_pause_periods_exactly_one_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1));

ALTER TABLE public.medication_pause_periods ADD CONSTRAINT chk_medication_pause_periods_interval CHECK (((started_at IS NULL) OR (ended_at IS NULL) OR (ended_at >= started_at)));

ALTER TABLE public.medication_pause_periods ADD CONSTRAINT chk_medication_pause_periods_legacy_context CHECK (((legacy_context AND ((reason)::text = 'reason_not_recorded'::text)) OR ((NOT legacy_context) AND ((reason)::text <> 'reason_not_recorded'::text) AND (started_at IS NOT NULL) AND ((recorded_by_membership_id IS NOT NULL) OR imported_context))));

ALTER TABLE public.medication_pause_periods ADD CONSTRAINT chk_medication_pause_periods_reason CHECK (((reason)::text = ANY (ARRAY[('out_of_supply'::character varying)::text, ('temporarily_not_needed'::character varying)::text, ('clinician_advice'::character varying)::text, ('side_effects'::character varying)::text, ('other'::character varying)::text, ('reason_not_recorded'::character varying)::text])));

ALTER TABLE public.medication_pause_periods ADD CONSTRAINT chk_medication_pause_periods_resuming_actor CHECK ((((ended_at IS NULL) AND (resumed_by_membership_id IS NULL)) OR ((ended_at IS NOT NULL) AND ((resumed_by_membership_id IS NOT NULL) OR imported_context))));

ALTER TABLE public.medication_takes ADD CONSTRAINT chk_medication_takes_exactly_one_source CHECK ((num_nonnulls(schedule_id, person_medication_id) = 1));

ALTER TABLE public.oauth_applications ADD CONSTRAINT chk_oauth_applications_token_auth_method CHECK (((((token_endpoint_auth_method)::text = 'none'::text) AND (NULLIF((client_secret)::text, ''::text) IS NULL) AND (NULLIF((client_secret_hash)::text, ''::text) IS NULL)) OR (((token_endpoint_auth_method)::text = ANY ((ARRAY['client_secret_basic'::character varying, 'client_secret_post'::character varying, 'client_secret_basic client_secret_post'::character varying])::text[])) AND ((NULLIF((client_secret)::text, ''::text) IS NOT NULL) OR (NULLIF((client_secret_hash)::text, ''::text) IS NOT NULL)))));

ALTER TABLE public.oauth_applications ADD CONSTRAINT oauth_client_kind CHECK (((client_kind)::text = ANY (ARRAY[('integration'::character varying)::text, ('mobile'::character varying)::text])));

ALTER TABLE public.oauth_grants ADD CONSTRAINT oauth_grant_authority_boundary CHECK (((((client_kind)::text = 'integration'::text) AND (household_membership_id IS NOT NULL) AND (person_id IS NOT NULL) AND (permissions_version IS NOT NULL)) OR (((client_kind)::text = 'mobile'::text) AND (household_membership_id IS NULL) AND (person_id IS NULL) AND (permissions_version IS NULL) AND (authenticated_at IS NOT NULL) AND (last_used_at IS NOT NULL))));

CREATE UNIQUE INDEX idx_dose_occurrence_person_medication_id_window ON public.medication_dose_occurrences USING btree (person_medication_id, window_starts_on, "position") WHERE (person_medication_id IS NOT NULL);

CREATE UNIQUE INDEX idx_dose_occurrence_schedule_id_window ON public.medication_dose_occurrences USING btree (schedule_id, window_starts_on, "position") WHERE (schedule_id IS NOT NULL);

CREATE UNIQUE INDEX idx_med_pause_periods_open_person_medication ON public.medication_pause_periods USING btree (person_medication_id) WHERE ((ended_at IS NULL) AND (person_medication_id IS NOT NULL));

CREATE UNIQUE INDEX idx_med_pause_periods_open_schedule ON public.medication_pause_periods USING btree (schedule_id) WHERE ((ended_at IS NULL) AND (schedule_id IS NOT NULL));

CREATE UNIQUE INDEX idx_on_household_membership_id_person_id_6ddc5a2882 ON public.person_access_grants USING btree (household_membership_id, person_id) WHERE (revoked_at IS NULL);

CREATE INDEX idx_on_pharmacologic_classes_df53f96090 ON public.medication_review_evidence_records USING gin (pharmacologic_classes);

CREATE UNIQUE INDEX idx_one_active_retention_hold_per_household ON public.household_retention_holds USING btree (household_id) WHERE ((status)::text = 'active'::text);

CREATE UNIQUE INDEX index_accounts_on_email ON public.accounts USING btree (email) WHERE (status = ANY (ARRAY[1, 2]));

CREATE INDEX index_accounts_on_preferences ON public.accounts USING gin (preferences);

CREATE UNIQUE INDEX index_dosages_one_adult_default ON public.dosages USING btree (medication_id) WHERE (default_for_adults = true);

CREATE UNIQUE INDEX index_dosages_one_child_default ON public.dosages USING btree (medication_id) WHERE (default_for_children = true);

CREATE UNIQUE INDEX index_health_event_medications_on_health_event_id_and_med_id ON public.health_event_medications USING btree (health_event_id, medication_id) WHERE (medication_id IS NOT NULL);

CREATE UNIQUE INDEX index_household_invitations_on_household_id_and_email ON public.household_invitations USING btree (household_id, email) WHERE ((accepted_at IS NULL) AND (revoked_at IS NULL));

CREATE UNIQUE INDEX index_household_memberships_on_person_id ON public.household_memberships USING btree (person_id) WHERE (person_id IS NOT NULL);

CREATE UNIQUE INDEX index_locations_on_household_id_and_lower_name ON public.locations USING btree (household_id, lower((name)::text)) WHERE (household_id IS NOT NULL);

CREATE INDEX index_locations_on_name_trigram ON public.locations USING gin (name public.gin_trgm_ops);

CREATE INDEX index_medication_review_evidence_records_on_candidate_terms ON public.medication_review_evidence_records USING gin (candidate_terms);

CREATE INDEX index_medication_review_evidence_records_on_interacting_terms ON public.medication_review_evidence_records USING gin (interacting_terms);

CREATE UNIQUE INDEX index_medication_takes_on_client_uuid ON public.medication_takes USING btree (client_uuid) WHERE (client_uuid IS NOT NULL);

CREATE UNIQUE INDEX index_medications_on_barcode ON public.medications USING btree (barcode) WHERE ((barcode IS NOT NULL) AND ((barcode)::text <> ''::text));

CREATE INDEX index_medications_on_barcode_trigram ON public.medications USING gin (barcode public.gin_trgm_ops);

CREATE INDEX index_medications_on_category_trigram ON public.medications USING gin (category public.gin_trgm_ops);

CREATE INDEX index_medications_on_dmd_code_trigram ON public.medications USING gin (dmd_code public.gin_trgm_ops);

CREATE INDEX index_medications_on_name_trigram ON public.medications USING gin (name public.gin_trgm_ops);

CREATE UNIQUE INDEX index_nhs_dmd_imports_one_active ON public.nhs_dmd_imports USING btree ((1)) WHERE (status = ANY (ARRAY[0, 1, 2, 3]));

CREATE UNIQUE INDEX index_people_on_email_present_unique ON public.people USING btree (email) WHERE ((email IS NOT NULL) AND (btrim((email)::text) <> ''::text));

CREATE UNIQUE INDEX index_people_on_household_id_and_account_id ON public.people USING btree (household_id, account_id) WHERE ((household_id IS NOT NULL) AND (account_id IS NOT NULL));

CREATE INDEX index_people_on_name_trigram ON public.people USING gin (name public.gin_trgm_ops);

CREATE UNIQUE INDEX index_person_medications_on_person_id_and_medication_id ON public.person_medications USING btree (person_id, medication_id) WHERE (retired_at IS NULL);

CREATE INDEX index_person_medications_on_person_id_and_position ON public.person_medications USING btree (person_id, "position");

CREATE INDEX index_schedules_on_schedule_config ON public.schedules USING gin (schedule_config);
