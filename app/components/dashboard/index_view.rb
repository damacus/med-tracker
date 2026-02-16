# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :people, :active_prescriptions, :upcoming_prescriptions, :url_helpers, :current_user, :doses

      def initialize(people:, active_prescriptions:, upcoming_prescriptions:, doses:, **options)
        @people = people
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @doses = doses
        @url_helpers = options[:url_helpers]
        @current_user = options[:current_user]
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section
          render_timeline_section
          render_prescriptions_section
        end
      end

      private

      def render_header
        div(class: 'flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4 mb-8') do
          Heading(level: 1) { t('dashboard.title') }
          render_quick_actions
        end
      end

      def render_quick_actions
        div(class: 'flex flex-row flex-wrap gap-2 sm:gap-3') do
          render RubyUI::Link.new(
            href: url_helpers&.new_medicine_path || '#',
            variant: :primary,
            class: 'min-h-[44px]'
          ) { t('dashboard.quick_actions.add_medicine') }
          render RubyUI::Link.new(
            href: url_helpers&.new_person_path || '#',
            variant: :secondary,
            class: 'min-h-[44px]'
          ) { t('dashboard.quick_actions.add_person') }
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 mb-8') do
          render Components::Dashboard::StatCard.new(title: t('dashboard.stats.people'), value: people.count,
                                                     icon_type: 'users')
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.active_prescriptions'),
            value: active_prescriptions.count,
            icon_type: 'pill'
          )
        end
      end

      def render_timeline_section
        div(class: 'space-y-4 mb-12') do
          Heading(level: 2) { t('dashboard.todays_schedule') }

          if doses.any?
            div(class: 'grid grid-cols-1 md:grid-cols-2 gap-4') do
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose)
              end
            end
          else
            render RubyUI::Card.new(class: 'p-8 text-center') do
              Text(weight: :muted) { t('dashboard.empty_state') }
            end
          end
        end
      end

      def render_prescriptions_section
        return if upcoming_prescriptions.empty?

        div(class: 'space-y-4') do
          Heading(level: 2) { t('dashboard.medication_schedule') }
          render_mobile_cards
          render_desktop_table
        end
      end

      def render_mobile_cards
        div(class: 'md:hidden space-y-3') do
          upcoming_prescriptions.each do |person, prescriptions|
            prescriptions.each do |prescription|
              render Components::Dashboard::PrescriptionCard.new(
                person: person,
                prescription: prescription,
                url_helpers: url_helpers,
                current_user: current_user
              )
            end
          end
        end
      end

      def render_desktop_table
        div(class: 'hidden md:block') do
          Table do
            TableHeader do
              TableRow do
                TableHead { t('dashboard.table.person') }
                TableHead { t('dashboard.table.medicine') }
                TableHead { t('dashboard.table.dosage') }
                TableHead { t('dashboard.table.quantity') }
                TableHead { t('dashboard.table.frequency') }
                TableHead { t('dashboard.table.end_date') }
                TableHead(class: 'text-center') { t('dashboard.table.actions') }
              end
            end

            TableBody do
              upcoming_prescriptions.each do |person, prescriptions|
                prescriptions.each do |prescription|
                  render Components::Dashboard::PrescriptionRow.new(
                    person: person,
                    prescription: prescription,
                    url_helpers: url_helpers,
                    current_user: current_user
                  )
                end
              end
            end
          end
        end
      end

      def render_empty_state
        div(class: 'space-y-6') do
          Heading(level: 2) { t('dashboard.medication_schedule') }
          Card(class: 'text-center py-12') do
            render RubyUI::CardContent.new do
              Text(size: '5', weight: 'semibold', class: 'text-slate-700 mb-2') do
                t('dashboard.no_active_prescriptions')
              end
              Text(class: 'text-slate-600') { t('dashboard.add_prescriptions_hint') }
            end
          end
        end
      end
    end
  end
end
