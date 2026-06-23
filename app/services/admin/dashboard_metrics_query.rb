# frozen_string_literal: true

module Admin
  class DashboardMetricsQuery
    include Rails.application.routes.url_helpers

    STALLED_IMPORT_THRESHOLD = 1.hour
    STALE_IMPORT_THRESHOLD = 35.days

    def call
      user_metrics
        .merge(people_metrics)
        .merge(schedule_metrics)
        .merge(invitation_metrics)
        .merge(activity_metrics)
        .merge(attention_items: attention_items)
    end

    private

    def user_metrics
      {
        total_users: household_users.count,
        active_users: household_users.active.count,
        recent_signups: household_users.where(created_at: 7.days.ago..).count,
        users_by_role: household_memberships.group(:role).count
      }
    end

    def people_metrics
      {
        total_people: household_people.count,
        people_by_type: household_people.group(:person_type).count,
        patients_without_carers: needing_carer_count
      }
    end

    def schedule_metrics
      {
        active_schedules: household_schedules.where(active: true).count
      }
    end

    def invitation_metrics
      {
        pending_invitations: household_invitations.pending.count,
        expired_invitations: expired_invitations_count
      }
    end

    def activity_metrics
      {
        recent_audit_events: recent_audit_events_count,
        recent_activity: recent_activity
      }
    end

    def needing_carer_count
      @needing_carer_count ||= household_people.needing_carer_assignment.count
    end

    def expired_invitations_count
      @expired_invitations_count ||= household_invitations.expired.count
    end

    def household_users
      @household_users ||=
        User.joins(person: { account: :household_memberships })
            .where(household_memberships: { household: Current.household })
            .distinct
    end

    def household_memberships
      @household_memberships ||= Current.household.household_memberships.active
    end

    def household_invitations
      @household_invitations ||= Current.household.household_invitations
    end

    def recent_audit_events_count
      audit_versions.where(created_at: 24.hours.ago..).count
    end

    def recent_activity
      audit_versions.order(created_at: :desc).limit(3).to_a
    end

    def household_people
      @household_people ||= Current.household.people
    end

    def household_schedules
      @household_schedules ||= Current.household.schedules
    end

    def audit_versions
      @audit_versions ||= PaperTrail::Version.where(household_id: Current.household.id)
    end

    def attention_items
      [carer_item, expired_invitations_item, dmd_item].compact
    end

    def carer_item
      return if needing_carer_count.zero?

      {
        severity: :high,
        title: I18n.t('admin.dashboard.attention.no_carers.title'),
        detail: I18n.t('admin.dashboard.attention.no_carers.detail', count: needing_carer_count),
        href: admin_people_path(route_options),
        action_label: I18n.t('admin.dashboard.attention.actions.view'),
        icon_type: 'activity'
      }
    end

    def expired_invitations_item
      return if expired_invitations_count.zero?

      {
        severity: :medium,
        title: I18n.t('admin.dashboard.attention.expired_invitations.title'),
        detail: I18n.t('admin.dashboard.attention.expired_invitations.detail', count: expired_invitations_count),
        href: admin_invitations_path(route_options),
        action_label: I18n.t('admin.dashboard.attention.actions.review'),
        icon_type: 'clock'
      }
    end

    def dmd_item
      import = latest_dmd_import
      return dmd_static_attention(:high, 'dmd_missing') if import.blank?

      dmd_failed_item(import) || dmd_stalled_item(import) || dmd_stale_item(import)
    end

    def latest_dmd_import
      return @latest_dmd_import if defined?(@latest_dmd_import)

      @latest_dmd_import = NhsDmdImport.latest_first.first
    end

    def dmd_attention(severity, key, timestamp)
      {
        severity: severity,
        title: I18n.t("admin.dashboard.attention.#{key}.title"),
        detail: I18n.t(
          "admin.dashboard.attention.#{key}.detail",
          when: ActionController::Base.helpers.time_ago_in_words(timestamp)
        ),
        href: new_admin_nhs_dmd_import_path(route_options),
        action_label: I18n.t('admin.dashboard.attention.actions.open'),
        icon_type: 'refresh_cw'
      }
    end

    def dmd_failed_item(import)
      return unless import.failed?

      dmd_attention(:high, 'dmd_failed', import.completed_at || import.created_at)
    end

    def dmd_stalled_item(import)
      return unless import.active?
      return unless import.created_at <= STALLED_IMPORT_THRESHOLD.ago

      dmd_attention(:medium, 'dmd_stalled', import.started_at || import.created_at)
    end

    def dmd_stale_item(import)
      return unless import.completed?
      return unless dmd_import_stale?(import)

      dmd_attention(:medium, 'dmd_stale', import.completed_at || import.updated_at)
    end

    def dmd_static_attention(severity, key)
      {
        severity: severity,
        title: I18n.t("admin.dashboard.attention.#{key}.title"),
        detail: I18n.t("admin.dashboard.attention.#{key}.detail"),
        href: new_admin_nhs_dmd_import_path(route_options),
        action_label: I18n.t('admin.dashboard.attention.actions.open'),
        icon_type: 'refresh_cw'
      }
    end

    def dmd_import_stale?(import)
      (import.completed_at || import.updated_at) <= STALE_IMPORT_THRESHOLD.ago
    end

    def route_options
      { household_slug: Current.household.slug }
    end
  end
end
