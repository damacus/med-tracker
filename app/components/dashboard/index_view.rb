# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :people, :active_prescriptions, :upcoming_prescriptions, :url_helpers, :current_user

      def initialize(people:, active_prescriptions:, upcoming_prescriptions:, url_helpers: nil, current_user: nil)
        @people = people
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @url_helpers = url_helpers
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section
          render_prescriptions_section
        end
      end

      private

      def render_header
        div(class: 'flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4 mb-8') do
          Heading(level: 1) { 'Dashboard' }
          render_quick_actions
        end
      end

      def render_quick_actions
        div(class: 'flex flex-row flex-wrap gap-2 sm:gap-3') do
          Link(
            href: url_helpers&.new_medicine_path || '#',
            class: "#{button_primary_classes} min-h-[44px]"
          ) { 'Add Medicine' }
          Link(
            href: url_helpers&.new_person_path || '#',
            class: "#{button_secondary_classes} min-h-[44px]"
          ) { 'Add Person' }
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 mb-8') do
          render Components::Dashboard::StatCard.new(title: 'People', value: people.count, icon_type: 'users')
          render Components::Dashboard::StatCard.new(title: 'Active Prescriptions', value: active_prescriptions.count,
                                                     icon_type: 'pill')
        end
      end

      def button_primary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-9 min-h-[36px] text-sm bg-primary text-primary-foreground shadow hover:bg-primary/90'
      end

      def button_secondary_classes
        'inline-flex items-center justify-center rounded-md font-medium transition-colors ' \
          'px-4 py-2 h-9 min-h-[36px] text-sm bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80'
      end

      def render_prescriptions_section
        return render_empty_state if upcoming_prescriptions.empty?

        div(class: 'space-y-4') do
          Heading(level: 2) { 'Medication Schedule' }
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
                TableHead { 'Person' }
                TableHead { 'Medicine' }
                TableHead { 'Dosage' }
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
