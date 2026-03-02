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
          div(data: { testid: 'admin-dashboard' }, class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl') do
            render_header
            render_metrics_grid
            render_quick_actions
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                'Admin Dashboard'
              end
            end
          end
        end

        # rubocop:disable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
        def render_metrics_grid
          div(class: 'grid grid-cols-2 lg:grid-cols-4 gap-4 mb-12') do
            render_metric_card(
              title: 'Total Users',
              value: metrics[:total_users] || 0,
              testid: 'metric-total-users',
              icon_type: 'users'
            )
            render_metric_card(
              title: 'Active Users',
              value: metrics[:active_users] || 0,
              testid: 'metric-active-users',
              icon_type: 'check'
            )
            render_metric_card(
              title: 'Recent Signups',
              value: metrics[:recent_signups] || 0,
              testid: 'metric-recent-signups',
              icon_type: 'activity'
            )
            render_metric_card(
              title: 'Total People',
              value: metrics[:total_people] || 0,
              testid: 'metric-total-people',
              icon_type: 'users'
            )
            render_metric_card(
              title: 'Active Schedules',
              value: metrics[:active_schedules] || 0,
              testid: 'metric-active-schedules',
              icon_type: 'pill'
            )
            render_metric_card(
              title: 'No Carers',
              value: metrics[:patients_without_carers] || 0,
              testid: 'metric-patients-without-carers',
              icon_type: 'activity',
              variant: metrics[:patients_without_carers]&.positive? ? :warning : :default
            )
          end
        end
        # rubocop:enable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity

        def render_metric_card(title:, value:, testid:, icon_type:, variant: :default)
          bg_color = variant == :warning ? 'bg-amber-50/50' : 'bg-white/50'
          border_color = variant == :warning ? 'border-amber-200' : 'border-none'

          Card(
            class: "#{border_color} shadow-sm #{bg_color} backdrop-blur-sm transition-all duration-300 " \
                   'hover:scale-[1.03] hover:shadow-xl hover:shadow-primary/5 cursor-default group',
            data: { testid: testid }
          ) do
            CardContent(class: 'p-6') do
              div(class: 'space-y-1') do
                div(class: 'flex items-center justify-between gap-2 mb-2 min-w-0') do
                  Text(
                    size: '1', weight: 'muted',
                    class: 'uppercase font-black tracking-widest group-hover:text-primary transition-colors truncate'
                  ) { title }
                  icon_classes = 'p-2 rounded-lg flex-shrink-0 ' \
                                 "#{icon_bg_class(icon_type, variant)} " \
                                 "#{value_color_class(icon_type, variant)} transition-colors"
                  div(class: icon_classes) do
                    render_icon(icon_type, size: 16)
                  end
                end
                div(class: 'flex items-baseline gap-2') do
                  span(class: "text-3xl font-black tracking-tight #{value_color_class(icon_type, variant)}",
                       data: { metric_value: value }) do
                    value.to_s
                  end
                end
              end
            end
          end
        end

        def render_icon(icon_type, size:)
          case icon_type
          when 'users' then render Icons::Users.new(size: size)
          when 'pill' then render Icons::Pill.new(size: size)
          when 'check' then render Icons::CheckCircle.new(size: size)
          when 'clock' then render Icons::Clock.new(size: size)
          else render Icons::Activity.new(size: size)
          end
        end

        def icon_bg_class(icon_type, variant)
          return 'bg-amber-100' if variant == :warning

          case icon_type
          when 'users' then 'bg-blue-50'
          when 'pill' then 'bg-emerald-50'
          when 'check' then 'bg-indigo-50'
          when 'clock' then 'bg-amber-50'
          else 'bg-slate-50'
          end
        end

        def value_color_class(icon_type, variant)
          return 'text-amber-700' if variant == :warning

          case icon_type
          when 'users' then 'text-blue-600'
          when 'pill' then 'text-emerald-600'
          when 'check' then 'text-indigo-600'
          when 'clock' then 'text-amber-600'
          else 'text-slate-900'
          end
        end

        def render_quick_actions
          div(class: 'space-y-6') do
            Heading(level: 2, size: '5', class: 'font-bold') { 'Quick Actions' }
            div(class: 'grid gap-4 sm:grid-cols-2 lg:grid-cols-4') do
              render_action_card(
                title: 'Manage Users',
                description: 'View and manage user accounts',
                href: '/admin/users',
                icon: Icons::Users.new(size: 24)
              )
              render_action_card(
                title: 'Invitations',
                description: 'Invite new users to join',
                href: '/admin/invitations',
                icon: Icons::User.new(size: 24)
              )
              render_action_card(
                title: 'Manage People',
                description: 'View and manage people records',
                href: '/people',
                icon: Icons::Users.new(size: 24)
              )
              render_action_card(
                title: 'Audit Trail',
                description: 'View system audit logs',
                href: '/admin/audit_logs',
                icon: Icons::Activity.new(size: 24)
              )
            end
          end
        end

        def render_action_card(title:, description:, href:, icon: nil)
          Link(
            href: href,
            variant: :ghost,
            class: 'flex flex-col gap-3 rounded-2xl bg-white p-6 border border-slate-100 shadow-sm ' \
                   'transition-all duration-300 hover:shadow-md hover:scale-[1.02] cursor-pointer no-underline h-full'
          ) do
            div(class: 'w-10 h-10 rounded-xl bg-slate-50 flex items-center justify-center text-slate-600') do
              render icon if icon
            end
            div(class: 'mt-2') do
              Heading(level: 3, size: '3', class: 'font-bold text-slate-900 mb-1') { title }
              Text(size: '2', weight: 'muted', class: 'leading-relaxed') { description }
            end
          end
        end
      end
    end
  end
end
