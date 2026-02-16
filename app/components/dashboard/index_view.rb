# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :people, :active_prescriptions, :upcoming_prescriptions, :url_helpers, :current_user, :doses

      def initialize(people:, active_prescriptions:, upcoming_prescriptions:, doses:, **options)
        url_helpers = options[:url_helpers]
        current_user = options[:current_user]
        @people = people
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @doses = doses
        @url_helpers = url_helpers
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section
          render_timeline_section
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
          Link(
            href: url_helpers&.new_medicine_path || '#',
            variant: :primary,
            class: 'min-h-[44px]'
          ) { t('dashboard.quick_actions.add_medicine') }
          Link(
            href: url_helpers&.new_person_path || '#',
            variant: :secondary,
            class: 'min-h-[44px]'
          ) { t('dashboard.quick_actions.add_person') }
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 mb-8') do
          render Components::Dashboard::StatCard.new(title: 'People', value: people.count, icon_type: 'users')
          render Components::Dashboard::StatCard.new(title: 'Active Prescriptions', value: active_prescriptions.count,
                                                     icon_type: 'pill')
        end
      end

      def render_timeline_section
        div(class: 'space-y-4') do
          Heading(level: 2) { 'Medication Schedule' }

          if doses.any?
            div(class: 'grid grid-cols-1 gap-4') do
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose)
              end
            end
          else
            render_empty_state
          end
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
                TableHead { 'Person' }
                TableHead { 'Medicine' }
                TableHead { 'Dosage' }
                TableHead { 'Quantity' }
                TableHead { 'Frequency' }
                TableHead { 'End Date' }
                TableHead(class: 'text-center') { 'Actions' }
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
          Heading(level: 2) { 'Medication Schedule' }
          Card(class: 'text-center py-12') do
            CardContent do
              Text(size: '5', weight: 'semibold', class: 'text-slate-700 mb-2') { 'No active prescriptions found' }
              Text(class: 'text-slate-600') { 'Add prescriptions to see them here' }
            end
          end
        end
      end
    end
  end
end
