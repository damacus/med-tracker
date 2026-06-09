# frozen_string_literal: true

module Admin
  class DashboardMetricsQuery
    STALLED_IMPORT_THRESHOLD = 1.hour

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
        total_users: User.count,
        active_users: User.active.count,
        recent_signups: User.where(created_at: 7.days.ago..).count,
        users_by_role: User.group(:role).count
      }
    end

    def people_metrics
      {
        total_people: Person.count,
        people_by_type: Person.group(:person_type).count,
        patients_without_carers: needing_carer_count
      }
    end

    def schedule_metrics
      {
        active_schedules: Schedule.where(active: true).count
      }
    end

    def invitation_metrics
      {
        pending_invitations: Invitation.pending.count,
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
      @needing_carer_count ||= Person.needing_carer_assignment.count
    end

    def expired_invitations_count
      @expired_invitations_count ||= Invitation.expired.count
    end

    def recent_audit_events_count
      PaperTrail::Version.where(created_at: 24.hours.ago..).count
    end

    def recent_activity
      PaperTrail::Version.order(created_at: :desc).limit(3).to_a
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
        href: '/admin/people',
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
        href: '/admin/invitations',
        action_label: I18n.t('admin.dashboard.attention.actions.review'),
        icon_type: 'clock'
      }
    end

    def dmd_item
      import = latest_dmd_import
      return if import.blank?

      if import.failed?
        dmd_attention(:high, 'dmd_failed', import.completed_at || import.created_at)
      elsif import.active? && import.created_at <= STALLED_IMPORT_THRESHOLD.ago
        dmd_attention(:medium, 'dmd_stalled', import.started_at || import.created_at)
      end
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
        href: '/admin/nhs_dmd_import/new',
        action_label: I18n.t('admin.dashboard.attention.actions.open'),
        icon_type: 'refresh_cw'
      }
    end
  end
end
