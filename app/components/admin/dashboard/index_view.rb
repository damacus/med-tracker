# frozen_string_literal: true

module Components
  module Admin
    module Dashboard
      class IndexView < Components::Base
        attr_reader :metrics

        def initialize(metrics: {})
          @metrics = metrics
          @actor_resolver = AuditActorResolver.new
          super()
        end

        def view_template
          div(data: { testid: 'admin-dashboard' },
              class: 'container mx-auto px-4 py-5 pb-24 md:py-6 md:pb-8 max-w-7xl space-y-6') do
            render_header
            render_attention_queue
            render_metrics_grid
            render_quick_actions
            render_recent_activity
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-center justify-between gap-3') do
            div do
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t('admin.dashboard.title')
              end
            end
            render_status_badge
          end
        end

        def render_status_badge
          clear = attention_items.empty?
          label = if clear
                    t('admin.dashboard.status.all_clear')
                  else
                    t('admin.dashboard.status.needs_attention', count: attention_items.size)
                  end
          classes =
            if clear
              'bg-success-container text-on-success-container'
            else
              'bg-warning-container text-on-warning-container'
            end

          span(
            data: { testid: 'dashboard-status' },
            class: "inline-flex items-center rounded-full px-4 py-1.5 text-sm font-bold #{classes}"
          ) { label }
        end

        def render_attention_queue
          section(data: { testid: 'attention-queue' }, class: 'space-y-2') do
            m3_heading(level: 2, size: '4', class: 'sr-only') { t('admin.dashboard.attention.title') }
            if attention_items.any?
              attention_items.each { |item| render_attention_row(item) }
            else
              render_attention_empty
            end
          end
        end

        def render_attention_row(item)
          a(
            href: item[:href],
            class: 'flex items-center gap-3 rounded-xl border border-border bg-card p-3 no-underline ' \
                   "shadow-sm transition-all hover:shadow-md #{severity_accent(item[:severity])}"
          ) do
            div(class: 'w-9 h-9 rounded-lg bg-secondary-container flex items-center ' \
                       'justify-center text-on-surface-variant shrink-0') do
              render attention_icon(item[:icon_type])
            end
            div(class: 'min-w-0 flex-1') do
              m3_text(class: 'font-bold text-foreground truncate') { item[:title] }
              m3_text(size: '2', weight: 'muted', class: 'truncate') { item[:detail] }
            end
            span(class: 'text-primary font-semibold text-sm shrink-0') { item[:action_label] }
          end
        end

        def render_attention_empty
          div(class: 'flex items-center gap-3 rounded-xl border border-border bg-card p-3 ' \
                     'shadow-sm border-l-4 border-l-success') do
            div(class: 'w-9 h-9 rounded-lg bg-success-container flex items-center justify-center ' \
                       'text-on-success-container shrink-0') do
              render Icons::Check.new(size: 20)
            end
            div(class: 'min-w-0') do
              m3_text(class: 'font-bold text-foreground') { t('admin.dashboard.attention.empty_title') }
              m3_text(size: '2', weight: 'muted') { t('admin.dashboard.attention.empty_detail') }
            end
          end
        end

        def attention_items
          metrics[:attention_items] || []
        end

        def severity_accent(severity)
          case severity&.to_sym
          when :high then 'border-l-4 border-l-error'
          when :medium then 'border-l-4 border-l-warning'
          else 'border-l-4 border-l-primary'
          end
        end

        def attention_icon(icon_type)
          case icon_type.to_s
          when 'clock' then Icons::Clock.new(size: 20)
          when 'refresh_cw' then Icons::RefreshCw.new(size: 20)
          when 'users' then Icons::Users.new(size: 20)
          when 'check' then Icons::Check.new(size: 20)
          when 'file_text' then Icons::FileText.new(size: 20)
          else Icons::Activity.new(size: 20)
          end
        end

        def render_metrics_grid
          div(class: 'grid grid-cols-2 lg:grid-cols-4 auto-rows-fr gap-3') do
            render_metric_card(title_key: 'admin.dashboard.metrics.total_users',
                               metric_key: :total_users,
                               testid: 'metric-total-users',
                               icon_type: 'users')
            render_metric_card(title_key: 'admin.dashboard.metrics.active_users',
                               metric_key: :active_users,
                               testid: 'metric-active-users',
                               icon_type: 'check')
            render_metric_card(title_key: 'admin.dashboard.metrics.total_people',
                               metric_key: :total_people,
                               testid: 'metric-total-people',
                               icon_type: 'users')
            render_metric_card(title_key: 'admin.dashboard.metrics.active_schedules',
                               metric_key: :active_schedules,
                               testid: 'metric-active-schedules',
                               icon_type: 'active_schedules')
            render_no_carers_metric
            render_metric_card(title_key: 'admin.dashboard.metrics_extra.pending_invitations',
                               metric_key: :pending_invitations,
                               testid: 'metric-pending-invitations',
                               icon_type: 'clock')
            render_metric_card(title_key: 'admin.dashboard.metrics_extra.recent_audit_events',
                               metric_key: :recent_audit_events,
                               testid: 'metric-recent-audit-events',
                               icon_type: 'activity')
            render_metric_card(title_key: 'admin.dashboard.metrics.recent_signups',
                               metric_key: :recent_signups,
                               testid: 'metric-recent-signups',
                               icon_type: 'activity')
          end
        end

        def render_metric_card(title_key:, metric_key:, testid:, icon_type:, variant: :default)
          value = metrics[metric_key] || 0
          render Components::Shared::MetricCard.new(
            title: t(title_key),
            value: value,
            icon_type: icon_type,
            testid: testid,
            variant: variant,
            value_data_attr: { metric_value: value },
            layout: :compact
          )
        end

        def render_no_carers_metric
          render_metric_card(title_key: 'admin.dashboard.metrics.no_carers',
                             metric_key: :patients_without_carers,
                             testid: 'metric-patients-without-carers',
                             icon_type: 'activity',
                             variant: (metrics[:patients_without_carers] || 0).positive? ? :warning : :default)
        end

        def render_quick_actions
          div(class: 'grid gap-4 md:grid-cols-2') do
            render_action_group(
              t('admin.dashboard.sections.user_access'),
              [
                ['manage_users', admin_users_path, Icons::Users],
                ['invitations', admin_invitations_path, Icons::User],
                ['household_settings', edit_admin_household_path, Icons::Settings]
              ]
            )
            render_action_group(
              t('admin.dashboard.sections.operations'),
              [
                ['manage_people', people_path, Icons::Users],
                ['audit_trail', admin_audit_logs_path, Icons::Activity],
                ['import_dmd', new_admin_nhs_dmd_import_path, Icons::RefreshCw]
              ]
            )
          end
        end

        def render_action_group(heading, actions)
          div(class: 'space-y-2') do
            m3_heading(level: 2, size: '4', class: 'font-bold') { heading }
            div(class: 'grid gap-3') do
              actions.each { |key, href, icon_class| render_action_card(key: key, href: href, icon_class: icon_class) }
            end
          end
        end

        def render_action_card(key:, href:, icon_class:)
          a(
            href: href,
            class: 'flex items-center gap-3 rounded-xl bg-card p-3 border border-border shadow-sm ' \
                   'transition-all hover:shadow-md no-underline min-w-0'
          ) do
            div(
              class: 'w-9 h-9 rounded-lg bg-secondary-container flex items-center ' \
                     'justify-center text-on-surface-variant shrink-0'
            ) do
              render icon_class.new(size: 24)
            end
            div(class: 'min-w-0') do
              m3_heading(level: 3, size: '3', class: 'font-bold text-foreground truncate') do
                t("admin.dashboard.quick_actions.#{key}_title")
              end
              m3_text(size: '2', weight: 'muted', class: 'truncate') do
                t("admin.dashboard.quick_actions.#{key}_description")
              end
            end
          end
        end

        def render_recent_activity
          versions = metrics[:recent_activity] || []
          section(data: { testid: 'dashboard-activity' }, class: 'space-y-2') do
            m3_heading(level: 2, size: '4', class: 'font-bold') { t('admin.dashboard.recent_activity.title') }
            if versions.any?
              div(class: 'grid gap-3') do
                versions.each { |version| render_activity_row(version) }
              end
            else
              m3_text(size: '2', weight: 'muted') { t('admin.dashboard.recent_activity.empty') }
            end
          end
        end

        def render_activity_row(version)
          if version.id
            a(href: admin_audit_log_path(version),
              class: 'flex items-center gap-3 rounded-xl bg-card p-3 border border-border shadow-sm ' \
                     'transition-all hover:shadow-md no-underline min-w-0') do
              render_activity_row_content(version)
            end
          else
            div(class: 'flex items-center gap-3 rounded-xl bg-card p-3 border border-border shadow-sm min-w-0') do
              render_activity_row_content(version)
            end
          end
        end

        def render_activity_row_content(version)
          m3_text(size: '2', weight: 'muted', class: 'font-mono shrink-0') do
            version.created_at.strftime('%H:%M')
          end
          m3_text(size: '2', class: 'text-foreground truncate') { activity_sentence(version) }
        end

        def activity_sentence(version)
          actor = @actor_resolver.name_for(version.whodunnit)
          verb = I18n.t(
            "admin.dashboard.recent_activity.events.#{version.event}",
            default: version.event.to_s.titleize
          )

          "#{actor} #{verb} #{version.item_type.titleize} ##{version.item_id}"
        end
      end
    end
  end
end
