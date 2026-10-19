ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_person_medication_household FOREIGN KEY (person_medication_id, household_id) REFERENCES public.person_medications(id, household_id) NOT VALID;

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_recorded_actor_household FOREIGN KEY (recorded_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_resumed_actor_household FOREIGN KEY (resumed_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;

ALTER TABLE ONLY public.medication_pause_periods
    ADD CONSTRAINT fk_med_pause_periods_schedule_household FOREIGN KEY (schedule_id, household_id) REFERENCES public.schedules(id, household_id) NOT VALID;

ALTER TABLE ONLY public.household_retention_holds
    ADD CONSTRAINT fk_rails_03ae12478f FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_rails_05ed1c349b FOREIGN KEY (patient_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_1789ad1477 FOREIGN KEY (reviewed_by_membership_id) REFERENCES public.household_memberships(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT fk_rails_1e2df17b6b FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_rails_280552daaa FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_417ee86956 FOREIGN KEY (evidence_record_id) REFERENCES public.medication_review_evidence_records(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.push_subscriptions
    ADD CONSTRAINT fk_rails_42e723e9f9 FOREIGN KEY (account_id) REFERENCES public.accounts(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_50c1621a30 FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_6a2170127e FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_6dd665ced2 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_72c4e175ed FOREIGN KEY (person_medication_id) REFERENCES public.person_medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_83e19c8bb8 FOREIGN KEY (primary_medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.schedules
    ADD CONSTRAINT fk_rails_87ac9a7f10 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_93516870c8 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT fk_rails_a1af1e497f FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.accounts
    ADD CONSTRAINT fk_accounts_person_id FOREIGN KEY (person_id) REFERENCES public.people(id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.dosages
    ADD CONSTRAINT fk_rails_a34bc544d1 FOREIGN KEY (medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.person_medications
    ADD CONSTRAINT fk_rails_a435932ea2 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.notification_events
    ADD CONSTRAINT fk_rails_ac049150f6 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.household_exports
    ADD CONSTRAINT fk_rails_af46b6b2a8 FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.household_purge_runs
    ADD CONSTRAINT fk_rails_af57a983cb FOREIGN KEY (household_id) REFERENCES public.households(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.carer_relationships
    ADD CONSTRAINT fk_rails_b2f8dc4c56 FOREIGN KEY (carer_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_takes
    ADD CONSTRAINT fk_rails_bac9dc4c0c FOREIGN KEY (schedule_id) REFERENCES public.schedules(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medications
    ADD CONSTRAINT fk_rails_d4b5830986 FOREIGN KEY (location_id) REFERENCES public.locations(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_rails_d7ad4ad8a3 FOREIGN KEY (interacting_medication_id) REFERENCES public.medications(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.location_memberships
    ADD CONSTRAINT fk_rails_e70491d63b FOREIGN KEY (location_id) REFERENCES public.locations(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.users
    ADD CONSTRAINT fk_rails_fa67535741 FOREIGN KEY (person_id) REFERENCES public.people(id) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_interacting_medication_household FOREIGN KEY (interacting_medication_id, household_id) REFERENCES public.medications(id, household_id) NOT VALID;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_primary_medication_household FOREIGN KEY (primary_medication_id, household_id) REFERENCES public.medications(id, household_id) NOT VALID;

ALTER TABLE ONLY public.medication_review_prompts
    ADD CONSTRAINT fk_review_prompts_reviewer_household FOREIGN KEY (reviewed_by_membership_id, household_id) REFERENCES public.household_memberships(id, household_id) NOT VALID;
