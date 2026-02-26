# frozen_string_literal: true

module Components
  module Admin
    module Dashboard
      class IndexView < Components::Base
        attr_reader :metrics

        def initialize(metrics: {})
          @metrics = metrics
          super()
        end

        def view_template
          div(data: { testid: 'admin-dashboard' }, class: 'container mx-auto px-4 py-8 space-y-8') do
            render_header
            render_metrics_grid
            render_quick_actions
          end
        end

        private

        def render_header
          header(class: 'space-y-2') do
            Heading(level: 1) { 'Admin Dashboard' }
            Text(weight: 'muted') { 'System overview and administrative tools' }
          end
        end

        # rubocop:disable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
        def render_metrics_grid
          div(class: 'grid gap-6 md:grid-cols-2 lg:grid-cols-3') do
            render_metric_card(
              title: 'Total Users',
              value: metrics[:total_users] || 0,
              testid: 'metric-total-users',
              icon: '👥'
            )
            render_metric_card(
              title: 'Active Users',
              value: metrics[:active_users] || 0,
              testid: 'metric-active-users',
              icon: '✅'
            )
            render_metric_card(
              title: 'Recent Signups',
              value: metrics[:recent_signups] || 0,
              testid: 'metric-recent-signups',
              icon: '🆕',
              subtitle: 'Last 7 days'
            )
            render_metric_card(
              title: 'Total People',
              value: metrics[:total_people] || 0,
              testid: 'metric-total-people',
              icon: '👤'
            )
            render_metric_card(
              title: 'Active Schedules',
              value: metrics[:active_schedules] || 0,
              testid: 'metric-active-schedules',
              icon: '💊'
            )
            render_metric_card(
              title: 'Patients Without Carers',
              value: metrics[:patients_without_carers] || 0,
              testid: 'metric-patients-without-carers',
              icon: '⚠️',
              variant: metrics[:patients_without_carers]&.positive? ? :warning : :default
            )
          end
        end
        # rubocop:enable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity

        # rubocop:disable Metrics/ParameterLists
        def render_metric_card(title:, value:, testid:, icon: nil, variant: :default, subtitle: nil)
          card_classes = base_card_classes(variant)

          div(class: card_classes, data: { testid: testid }) do
            div(class: 'flex items-center justify-between') do
              div do
                Text(size: '2', weight: 'medium', class: 'text-slate-600') { title }
                Text(size: '7', weight: 'bold', class: 'mt-2', data: { metric_value: value }) { value.to_s }
                Text(size: '1', weight: 'muted', class: 'mt-1') { subtitle } if subtitle
              end
              div(class: 'text-4xl') { icon } if icon
            end
          end
        end
        # rubocop:enable Metrics/ParameterLists

        def base_card_classes(variant)
          base = 'rounded-xl border bg-white p-6 shadow-sm'
          case variant
          when :warning
            "#{base} border-amber-200 bg-amber-50"
          else
            "#{base} border-slate-200"
          end
        end

        def render_quick_actions
          Card do
            CardHeader do
              Heading(level: 2, size: '4', class: 'font-semibold leading-none tracking-tight') do
                'Quick Actions'
              end
            end
            CardContent do
              div(class: 'grid gap-4 sm:grid-cols-2 lg:grid-cols-3') do
                render_action_link(
                  title: 'Manage Users',
                  description: 'View and manage user accounts',
                  href: '/admin/users',
                  icon: '👥'
                )
                render_action_link(
                  title: 'Invitations',
                  description: 'Invite new users to join MedTracker',
                  href: '/admin/invitations',
                  icon: '✉️'
                )
                render_action_link(
                  title: 'Manage People',
                  description: 'View and manage people records',
                  href: '/people',
                  icon: '👤'
                )
                render_action_link(
                  title: 'Audit Trail',
                  description: 'View system audit logs and change history',
                  href: '/admin/audit_logs',
                  icon: '📋'
                )
              end
            end
          end
        end

        def render_action_link(title:, description:, href:, icon: nil)
          Link(
            href: href,
            variant: :ghost,
            class: 'flex items-start gap-4 rounded-lg border border-slate-200 p-4 h-auto ' \
                   'transition-colors hover:bg-slate-50 hover:border-slate-300'
          ) do
            div(class: 'text-3xl') { icon } if icon
            div do
              Text(size: '4', weight: 'semibold', class: 'text-slate-900') { title }
              Text(size: '2', weight: 'muted', class: 'mt-1') { description }
            end
          end
        end
      end
    end
  end
end
