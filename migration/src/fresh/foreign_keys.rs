use sea_orm_migration::prelude::*;

pub async fn create(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_carer_relationships_carer_household")
                .from(
                    Alias::new("carer_relationships"),
                    (Alias::new("carer_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_carer_relationships_household")
                .from(
                    Alias::new("carer_relationships"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_carer_relationships_patient_household")
                .from(
                    Alias::new("carer_relationships"),
                    (Alias::new("patient_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_dosages_medication_id_household")
                .from(
                    Alias::new("dosages"),
                    (Alias::new("medication_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_dose_occurrence_medication_take_id_household")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    (Alias::new("medication_take_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("medication_takes"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_dose_occurrence_person_medication_id_household")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    (
                        Alias::new("person_medication_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("person_medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_dose_occurrence_resolved_by_membership_id_household")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    (
                        Alias::new("resolved_by_membership_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("household_memberships"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_dose_occurrence_schedule_id_household")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    (Alias::new("schedule_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("schedules"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_health_event_medications_health_event_id_household")
                .from(
                    Alias::new("health_event_medications"),
                    (Alias::new("health_event_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("health_events"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_health_event_medications_medication_id_household")
                .from(
                    Alias::new("health_event_medications"),
                    (Alias::new("medication_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_health_events_person_id_household")
                .from(
                    Alias::new("health_events"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_household_invitation_grants_invitation_household")
                .from(
                    Alias::new("household_invitation_grants"),
                    (
                        Alias::new("household_invitation_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("household_invitations"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_household_invitation_grants_person_household")
                .from(
                    Alias::new("household_invitation_grants"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_household_memberships_person_id_household")
                .from(
                    Alias::new("household_memberships"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_location_memberships_location_id_household")
                .from(
                    Alias::new("location_memberships"),
                    (Alias::new("location_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("locations"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_location_memberships_person_id_household")
                .from(
                    Alias::new("location_memberships"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medication_takes_person_medication_id_household")
                .from(
                    Alias::new("medication_takes"),
                    (
                        Alias::new("person_medication_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("person_medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medication_takes_schedule_id_household")
                .from(
                    Alias::new("medication_takes"),
                    (Alias::new("schedule_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("schedules"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medication_takes_taken_from_location_id_household")
                .from(
                    Alias::new("medication_takes"),
                    (
                        Alias::new("taken_from_location_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("locations"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medication_takes_taken_from_medication_id_household")
                .from(
                    Alias::new("medication_takes"),
                    (
                        Alias::new("taken_from_medication_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medications_created_by_membership_id_household")
                .from(
                    Alias::new("medications"),
                    (
                        Alias::new("created_by_membership_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("household_memberships"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_medications_location_id_household")
                .from(
                    Alias::new("medications"),
                    (Alias::new("location_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("locations"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_notification_events_person_id_household")
                .from(
                    Alias::new("notification_events"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_notification_preferences_person_id_household")
                .from(
                    Alias::new("notification_preferences"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_access_grants_carer_relationship_household")
                .from(
                    Alias::new("person_access_grants"),
                    (
                        Alias::new("carer_relationship_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("carer_relationships"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_access_grants_granted_by_membership_id_household")
                .from(
                    Alias::new("person_access_grants"),
                    (
                        Alias::new("granted_by_membership_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("household_memberships"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_access_grants_household_membership_id_household")
                .from(
                    Alias::new("person_access_grants"),
                    (
                        Alias::new("household_membership_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("household_memberships"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_access_grants_person_id_household")
                .from(
                    Alias::new("person_access_grants"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_medications_medication_id_household")
                .from(
                    Alias::new("person_medications"),
                    (Alias::new("medication_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_medications_person_id_household")
                .from(
                    Alias::new("person_medications"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_person_medications_source_dosage_option_id_household")
                .from(
                    Alias::new("person_medications"),
                    (
                        Alias::new("source_dosage_option_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("dosages"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_008d37b889")
                .from(
                    Alias::new("health_event_medications"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_00db5e06d0")
                .from(Alias::new("health_events"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_0f321a0531")
                .from(
                    Alias::new("api_sessions"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_1018073a20")
                .from(Alias::new("api_sessions"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_13d7545ba0")
                .from(
                    Alias::new("medication_pause_periods"),
                    Alias::new("recorded_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_15b046bfbe")
                .from(Alias::new("versions"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_1751f12354")
                .from(Alias::new("oauth_grants"), Alias::new("person_id"))
                .to(Alias::new("people"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_19c49f71b4")
                .from(Alias::new("locations"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_1a6ff9b7fd")
                .from(Alias::new("person_medications"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_1ba042dadc")
                .from(
                    Alias::new("nhs_dmd_trade_families"),
                    Alias::new("trade_family_group_id"),
                )
                .to(Alias::new("nhs_dmd_trade_family_groups"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_1f0c478552")
                .from(
                    Alias::new("medications"),
                    Alias::new("created_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_211c1cecac")
                .from(Alias::new("oauth_applications"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_2586b16017")
                .from(
                    Alias::new("account_webauthn_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_27447b69f8")
                .from(
                    Alias::new("audit_export_deliveries"),
                    Alias::new("audit_ledger_entry_id"),
                )
                .to(Alias::new("audit_ledger_entries"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_28fa9af012")
                .from(
                    Alias::new("audit_ledger_entries"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_2d519987a4")
                .from(
                    Alias::new("account_login_failures"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_35043a57c0")
                .from(Alias::new("api_idempotency_keys"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_3adf622f08")
                .from(
                    Alias::new("api_household_selection_grants"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_3d02d8db1e")
                .from(
                    Alias::new("medication_pause_periods"),
                    Alias::new("person_medication_id"),
                )
                .to(Alias::new("person_medications"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_3db380435a")
                .from(
                    Alias::new("medication_takes"),
                    Alias::new("taken_from_location_id"),
                )
                .to(Alias::new("locations"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_3dd9aab894")
                .from(
                    Alias::new("nhs_dmd_amp_trade_families"),
                    Alias::new("trade_family_id"),
                )
                .to(Alias::new("nhs_dmd_trade_families"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_3e095b0b7e")
                .from(Alias::new("oauth_grants"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_420e071daa")
                .from(Alias::new("schedules"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_448c435863")
                .from(Alias::new("api_change_events"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_4b7b1ea600")
                .from(
                    Alias::new("health_event_medications"),
                    Alias::new("medication_id"),
                )
                .to(Alias::new("medications"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_4cd0a97e69")
                .from(
                    Alias::new("medication_pause_periods"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_4e7357c544")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    Alias::new("resolved_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_5316070ef5")
                .from(Alias::new("api_tombstones"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_53df2ee65d")
                .from(Alias::new("person_access_grants"), Alias::new("person_id"))
                .to(Alias::new("people"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_572f0022f4")
                .from(
                    Alias::new("household_exports"),
                    Alias::new("requested_by_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_599c66dae5")
                .from(
                    Alias::new("household_purge_runs"),
                    Alias::new("requested_by_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_5a60bf71ce")
                .from(Alias::new("api_app_tokens"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_6219f3f23e")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    Alias::new("medication_take_id"),
                )
                .to(Alias::new("medication_takes"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_649e7a6bdb")
                .from(Alias::new("dosages"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_6b4e5876ef")
                .from(
                    Alias::new("notification_preferences"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_742fee62ad")
                .from(
                    Alias::new("account_webauthn_user_ids"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_75bd8b3918")
                .from(
                    Alias::new("security_audit_events"),
                    Alias::new("actor_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_78f10950ce")
                .from(
                    Alias::new("person_medications"),
                    Alias::new("source_dosage_option_id"),
                )
                .to(Alias::new("dosages"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_7bb88a451f")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    Alias::new("person_medication_id"),
                )
                .to(Alias::new("person_medications"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_823f8d7a81")
                .from(Alias::new("account_otp_keys"), Alias::new("id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_829e856eff")
                .from(Alias::new("people"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_82a2dd6e6d")
                .from(
                    Alias::new("medication_pause_periods"),
                    Alias::new("schedule_id"),
                )
                .to(Alias::new("schedules"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_83f9a515c3")
                .from(Alias::new("household_memberships"), Alias::new("person_id"))
                .to(Alias::new("people"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_84d8e101dd")
                .from(Alias::new("platform_admins"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_8513a75db1")
                .from(
                    Alias::new("medication_pause_periods"),
                    Alias::new("resumed_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_88300b88fe")
                .from(
                    Alias::new("schedules"),
                    Alias::new("source_dosage_option_id"),
                )
                .to(Alias::new("dosages"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_8868aa7ac0")
                .from(Alias::new("account_identities"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .on_delete(ForeignKeyAction::Cascade)
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_90265af5fd")
                .from(Alias::new("api_tombstones"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_932cf78829")
                .from(
                    Alias::new("household_retention_holds"),
                    Alias::new("released_by_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_95551392d9")
                .from(
                    Alias::new("support_access_sessions"),
                    Alias::new("platform_admin_id"),
                )
                .to(Alias::new("platform_admins"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_993965df05")
                .from(
                    Alias::new("active_storage_variant_records"),
                    Alias::new("blob_id"),
                )
                .to(Alias::new("active_storage_blobs"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_9f635195b2")
                .from(
                    Alias::new("api_idempotency_keys"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a04c18169e")
                .from(
                    Alias::new("person_access_grants"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a0a56faf23")
                .from(
                    Alias::new("api_app_tokens"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a25b9abfde")
                .from(
                    Alias::new("audit_checkpoints"),
                    Alias::new("audit_signing_key_id"),
                )
                .to(Alias::new("audit_signing_keys"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a311021dea")
                .from(Alias::new("audit_chain_heads"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a31f64e500")
                .from(
                    Alias::new("api_idempotency_keys"),
                    Alias::new("api_session_id"),
                )
                .to(Alias::new("api_sessions"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a4c5b33ad6")
                .from(
                    Alias::new("api_idempotency_keys"),
                    Alias::new("api_app_token_id"),
                )
                .to(Alias::new("api_app_tokens"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a8444d0c26")
                .from(
                    Alias::new("household_invitation_grants"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_a87c7f4df1")
                .from(
                    Alias::new("api_change_events"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_ad685ca12a")
                .from(Alias::new("medication_takes"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_af16ab68d1")
                .from(
                    Alias::new("oauth_grants"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_af53a77073")
                .from(Alias::new("api_change_events"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_b436553447")
                .from(Alias::new("audit_checkpoints"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_b5fe564bc0")
                .from(
                    Alias::new("location_memberships"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_b9ce8e6e8e")
                .from(
                    Alias::new("households"),
                    Alias::new("created_by_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_bc150fd06f")
                .from(
                    Alias::new("security_audit_events"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_be17586637")
                .from(
                    Alias::new("health_event_medications"),
                    Alias::new("health_event_id"),
                )
                .to(Alias::new("health_events"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c12e6f729d")
                .from(
                    Alias::new("household_retention_holds"),
                    Alias::new("approved_by_account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c3627bca9a")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    Alias::new("schedule_id"),
                )
                .to(Alias::new("schedules"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c3b3935057")
                .from(
                    Alias::new("active_storage_attachments"),
                    Alias::new("blob_id"),
                )
                .to(Alias::new("active_storage_blobs"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c453d62529")
                .from(
                    Alias::new("api_tombstones"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c5c7a8fb9b")
                .from(
                    Alias::new("account_password_reset_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c6a1fbd2bf")
                .from(
                    Alias::new("active_storage_attachments"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_c9deefabbe")
                .from(
                    Alias::new("audit_export_deliveries"),
                    Alias::new("audit_checkpoint_id"),
                )
                .to(Alias::new("audit_checkpoints"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_cbf5dd79c9")
                .from(
                    Alias::new("household_invitation_grants"),
                    Alias::new("household_invitation_id"),
                )
                .to(Alias::new("household_invitations"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_cdedf5be2c")
                .from(
                    Alias::new("account_active_session_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_ce4cdc7516")
                .from(Alias::new("health_events"), Alias::new("person_id"))
                .to(Alias::new("people"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_d2daa8499a")
                .from(
                    Alias::new("household_invitation_grants"),
                    Alias::new("person_id"),
                )
                .to(Alias::new("people"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_d5addd7cc9")
                .from(
                    Alias::new("oauth_grants"),
                    Alias::new("oauth_application_id"),
                )
                .to(Alias::new("oauth_applications"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_d7d088ec4c")
                .from(Alias::new("account_lockouts"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_d8b00bbe35")
                .from(
                    Alias::new("support_access_sessions"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_dcc0aec101")
                .from(
                    Alias::new("household_invitations"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_de619a6e71")
                .from(
                    Alias::new("person_access_grants"),
                    Alias::new("household_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_dee524eb5b")
                .from(
                    Alias::new("medication_dose_occurrences"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_e0faca3a58")
                .from(
                    Alias::new("medication_takes"),
                    Alias::new("taken_from_medication_id"),
                )
                .to(Alias::new("medications"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_e3e4bfb5dc")
                .from(Alias::new("native_device_tokens"), Alias::new("account_id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_e65835dcbc")
                .from(
                    Alias::new("account_login_change_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_e7dc011d7c")
                .from(
                    Alias::new("household_memberships"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_eb691e7e17")
                .from(
                    Alias::new("account_verification_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_ef43948844")
                .from(Alias::new("account_recovery_codes"), Alias::new("id"))
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_f11dbb5a09")
                .from(
                    Alias::new("household_memberships"),
                    Alias::new("household_id"),
                )
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_f34d593da6")
                .from(Alias::new("medications"), Alias::new("household_id"))
                .to(Alias::new("households"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_f7412b2e4d")
                .from(
                    Alias::new("household_invitations"),
                    Alias::new("invited_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_fa67029c99")
                .from(
                    Alias::new("account_remember_keys"),
                    Alias::new("account_id"),
                )
                .to(Alias::new("accounts"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_rails_fcbf4f0da8")
                .from(
                    Alias::new("person_access_grants"),
                    Alias::new("granted_by_membership_id"),
                )
                .to(Alias::new("household_memberships"), Alias::new("id"))
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_schedules_medication_id_household")
                .from(
                    Alias::new("schedules"),
                    (Alias::new("medication_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("medications"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_schedules_person_id_household")
                .from(
                    Alias::new("schedules"),
                    (Alias::new("person_id"), Alias::new("household_id")),
                )
                .to(
                    Alias::new("people"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("fk_schedules_source_dosage_option_id_household")
                .from(
                    Alias::new("schedules"),
                    (
                        Alias::new("source_dosage_option_id"),
                        Alias::new("household_id"),
                    ),
                )
                .to(
                    Alias::new("dosages"),
                    (Alias::new("id"), Alias::new("household_id")),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("oauth_grant_application_kind")
                .from(
                    Alias::new("oauth_grants"),
                    (
                        Alias::new("oauth_application_id"),
                        Alias::new("client_kind"),
                    ),
                )
                .to(
                    Alias::new("oauth_applications"),
                    (Alias::new("id"), Alias::new("client_kind")),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}
