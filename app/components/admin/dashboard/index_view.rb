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
            h1(class: 'text-3xl font-semibold text-slate-900') { 'Admin Dashboard' }
            p(class: 'text-slate-600') { 'System overview and administrative tools' }
          end
        end

        def render_metrics_grid
          div(class: 'grid gap-6 md:grid-cols-2 lg:grid-cols-4') do
            render_metric_card(
              title: 'Total Users',
              value: metrics[:total_users] || 0,
              testid: 'metric-total-users',
              icon: '👥'
            )
            render_metric_card(
              title: 'Total People',
              value: metrics[:total_people] || 0,
              testid: 'metric-total-people',
              icon: '👤'
            )
            render_metric_card(
              title: 'Active Prescriptions',
              value: metrics[:active_prescriptions] || 0,
              testid: 'metric-active-prescriptions',
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

        def render_metric_card(title:, value:, testid:, icon: nil, variant: :default)
          card_classes = base_card_classes(variant)

          div(class: card_classes, data: { testid: testid }) do
            div(class: 'flex items-center justify-between') do
              div do
                p(class: 'text-sm font-medium text-slate-600') { title }
                p(class: 'text-3xl font-bold text-slate-900 mt-2', data: { metric_value: value }) { value.to_s }
              end
              div(class: 'text-4xl') { icon } if icon
            end
          end
        end

        def base_card_classes(variant)
          base = 'rounded-xl border bg-white p-6 shadow-sm'
          case variant
          when :warning
            "#{base} border-amber-200 bg-amber-50"
          else
            "#{base} border-slate-200"
          end
        end

        def render_placeholder
          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6') do
            render_card(
              title: 'User Management',
              description: 'Review and manage user accounts and access levels',
              icon: '👥',
              href: admin_users_path
            )

            render_card(
              title: 'Audit Trail',
              description: 'View security audit logs of sensitive actions',
              icon: '📋',
              href: admin_audit_logs_path
            )
          end
        end

        def render_card(title:, description:, icon:, href:)
          link_to(href, class: 'block transition hover:scale-[1.02]') do
            Card(class: 'h-full') do
              CardHeader do
                div(class: 'flex items-center gap-3') do
                  div(class: 'text-3xl') { icon }
                  div do
                    CardTitle(class: 'text-xl') { title }
                    CardDescription { description }
                  end
                end
              end
            end
            CardContent do
              div(class: 'grid gap-4 sm:grid-cols-2') do
                render_action_link(
                  title: 'Manage Users',
                  description: 'View and manage user accounts',
                  href: '/admin/users',
                  icon: '👥'
                )
                render_action_link(
                  title: 'Manage People',
                  description: 'View and manage people records',
                  href: '/people',
                  icon: '👤'
                )
              end
            end
          end
        end

        def render_action_link(title:, description:, href:, icon: nil)
          a(
            href: href,
            class: 'flex items-start gap-4 rounded-lg border border-slate-200 p-4 ' \
                   'transition-colors hover:bg-slate-50 hover:border-slate-300'
          ) do
            div(class: 'text-3xl') { icon } if icon
            div do
              h3(class: 'font-semibold text-slate-900') { title }
              p(class: 'text-sm text-slate-600 mt-1') { description }
            end
          end
        end
      end
    end
  end
end
