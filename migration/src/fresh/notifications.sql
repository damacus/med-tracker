DROP POLICY household_tenant_isolation ON public.notification_preferences;
CREATE POLICY recipient_delivery_settings ON public.notification_preferences
USING (account_id=med_tracker.current_account_id() AND household_id=med_tracker.current_household_id())
WITH CHECK (account_id=med_tracker.current_account_id() AND household_id=med_tracker.current_household_id());

CREATE TABLE public.household_notification_subscriptions (
  account_id bigint NOT NULL REFERENCES public.accounts(id),
  household_id bigint NOT NULL REFERENCES public.households(id),
  person_id bigint NOT NULL,
  enabled boolean NOT NULL DEFAULT false,
  created_at timestamp(6) without time zone NOT NULL DEFAULT timezone('UTC',clock_timestamp()),
  updated_at timestamp(6) without time zone NOT NULL DEFAULT timezone('UTC',clock_timestamp()),
  PRIMARY KEY(account_id,household_id,person_id),
  FOREIGN KEY(person_id,household_id) REFERENCES public.household_memberships(person_id,household_id)
);
ALTER TABLE public.household_notification_subscriptions OWNER TO med_tracker_owner;
ALTER TABLE public.household_notification_subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.household_notification_subscriptions FORCE ROW LEVEL SECURITY;
GRANT SELECT,INSERT,UPDATE,DELETE ON public.household_notification_subscriptions TO med_tracker_app;
CREATE POLICY recipient_subscription ON public.household_notification_subscriptions
USING (account_id=med_tracker.current_account_id() AND household_id=med_tracker.current_household_id())
WITH CHECK (account_id=med_tracker.current_account_id() AND household_id=med_tracker.current_household_id()
  AND EXISTS(SELECT 1 FROM public.household_memberships m WHERE m.household_id=household_notification_subscriptions.household_id
    AND m.person_id=household_notification_subscriptions.person_id AND m.status='active' AND m.revoked_at IS NULL));
