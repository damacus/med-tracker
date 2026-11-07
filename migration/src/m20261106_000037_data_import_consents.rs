use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut table = Table::create();
        table.table(Alias::new("data_import_consents"));
        table.col(
            ColumnDef::new(Alias::new("id"))
                .string()
                .not_null()
                .primary_key(),
        );
        for name in [
            "household_id",
            "requester_account_id",
            "requester_membership_id",
            "recipient_account_id",
            "recipient_membership_id",
            "target_person_id",
        ] {
            table.col(ColumnDef::new(Alias::new(name)).big_integer().not_null());
        }
        table.col(
            ColumnDef::new(Alias::new("source_sha256"))
                .string()
                .not_null(),
        );
        table.col(
            ColumnDef::new(Alias::new("source_person"))
                .json_binary()
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("decision")).string().not_null());
        table.col(
            ColumnDef::new(Alias::new("created_at"))
                .timestamp()
                .not_null(),
        );
        table.col(ColumnDef::new(Alias::new("decided_at")).timestamp());
        for (column, target) in [
            ("household_id", "households"),
            ("requester_account_id", "accounts"),
            ("recipient_account_id", "accounts"),
            ("requester_membership_id", "household_memberships"),
            ("recipient_membership_id", "household_memberships"),
            ("target_person_id", "people"),
        ] {
            table.foreign_key(
                ForeignKey::create()
                    .from(Alias::new("data_import_consents"), Alias::new(column))
                    .to(Alias::new(target), Alias::new("id")),
            );
        }
        manager.create_table(table.to_owned()).await?;
        // SeaQuery has no PostgreSQL RLS policy or role GRANT builders.
        manager.get_connection().execute_unprepared("ALTER TABLE public.data_import_consents ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.data_import_consents FORCE ROW LEVEL SECURITY;
CREATE POLICY import_consent_actor_active ON public.data_import_consents AS RESTRICTIVE FOR ALL TO med_tracker_app USING (EXISTS (SELECT 1 FROM public.household_memberships actor JOIN public.accounts a ON a.person_id = actor.person_id JOIN public.households h ON h.id = actor.household_id WHERE actor.id = med_tracker.current_membership_id() AND actor.household_id = med_tracker.current_household_id() AND actor.status = 'active' AND actor.revoked_at IS NULL AND a.id = med_tracker.current_account_id() AND a.active AND a.status = 2 AND h.status = 'active' AND h.lifecycle_state = 'active' AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = a.id AND l.deadline > pg_catalog.timezone('UTC', pg_catalog.clock_timestamp())))) WITH CHECK (EXISTS (SELECT 1 FROM public.household_memberships actor JOIN public.accounts a ON a.person_id = actor.person_id JOIN public.households h ON h.id = actor.household_id WHERE actor.id = med_tracker.current_membership_id() AND actor.household_id = med_tracker.current_household_id() AND actor.status = 'active' AND actor.revoked_at IS NULL AND a.id = med_tracker.current_account_id() AND a.active AND a.status = 2 AND h.status = 'active' AND h.lifecycle_state = 'active' AND NOT EXISTS (SELECT 1 FROM public.account_lockouts l WHERE l.account_id = a.id AND l.deadline > pg_catalog.timezone('UTC', pg_catalog.clock_timestamp()))));
CREATE POLICY import_consent_select ON public.data_import_consents FOR SELECT TO med_tracker_app USING (household_id = med_tracker.current_household_id() AND (requester_account_id = med_tracker.current_account_id() AND requester_membership_id = med_tracker.current_membership_id() OR recipient_account_id = med_tracker.current_account_id() AND recipient_membership_id = med_tracker.current_membership_id()));
CREATE POLICY import_consent_insert ON public.data_import_consents FOR INSERT TO med_tracker_app WITH CHECK (household_id = med_tracker.current_household_id() AND requester_account_id = med_tracker.current_account_id() AND requester_membership_id = med_tracker.current_membership_id() AND requester_account_id <> recipient_account_id AND decision = 'pending' AND decided_at IS NULL AND EXISTS (SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.person_id = m.person_id WHERE m.id = data_import_consents.recipient_membership_id AND m.household_id = data_import_consents.household_id AND m.person_id = data_import_consents.target_person_id AND m.status = 'active' AND m.revoked_at IS NULL AND a.id = data_import_consents.recipient_account_id));
CREATE POLICY import_consent_update ON public.data_import_consents FOR UPDATE TO med_tracker_app USING (household_id = med_tracker.current_household_id() AND recipient_account_id = med_tracker.current_account_id() AND recipient_membership_id = med_tracker.current_membership_id() AND decision IN ('pending', 'approved')) WITH CHECK (household_id = med_tracker.current_household_id() AND recipient_account_id = med_tracker.current_account_id() AND recipient_membership_id = med_tracker.current_membership_id() AND decision IN ('approved', 'rejected') AND decided_at IS NOT NULL);
CREATE POLICY import_consent_requester_lock ON public.data_import_consents FOR UPDATE TO med_tracker_app USING (household_id = med_tracker.current_household_id() AND requester_account_id = med_tracker.current_account_id() AND requester_membership_id = med_tracker.current_membership_id()) WITH CHECK (false);
REVOKE ALL PRIVILEGES ON public.data_import_consents FROM med_tracker_app;
GRANT SELECT, INSERT ON public.data_import_consents TO med_tracker_app;
GRANT UPDATE (decision, decided_at) ON public.data_import_consents TO med_tracker_app;").await?;
        Ok(())
    }
}
