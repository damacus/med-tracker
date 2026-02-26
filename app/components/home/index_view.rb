# frozen_string_literal: true

module Components
  module Home
    class IndexView < Components::Base
      def view_template
        div(class: 'container mx-auto px-4 py-8') do
          div(class: 'mb-8 text-center') do
            Heading(level: 1, class: 'mb-2') { 'Medication Tracker' }
            Text(size: '4', weight: 'muted') { 'Welcome to your personal medication tracking application.' }
          end

          div(class: 'grid grid-cols-1 md:grid-cols-2 gap-6 max-w-4xl mx-auto') do
            medications_card
            people_card
          end
        end
      end

      private

      def medications_card
        Link(href: medications_path, variant: :ghost, class: 'block transition-transform hover:scale-105 h-auto p-0') do
          Card(class: 'h-full') do
            CardHeader do
              div(class: 'w-12 h-12 rounded-xl flex items-center justify-center bg-blue-100 text-blue-700 mb-2') do
                render_medication_icon
              end
            end
            CardContent(class: 'space-y-2') do
              Heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') { 'Medications' }
              CardDescription { 'Manage your medication inventory, track stock levels, and view dosage information' }
            end
          end
        end
      end

      def people_card
        Link(href: people_path, variant: :ghost, class: 'block transition-transform hover:scale-105 h-auto p-0') do
          Card(class: 'h-full') do
            CardHeader do
              div(class: 'w-12 h-12 rounded-xl flex items-center justify-center bg-violet-100 text-violet-700 mb-2') do
                render_people_icon
              end
            end
            CardContent(class: 'space-y-2') do
              Heading(level: 2, size: '6', class: 'font-semibold leading-none tracking-tight') { 'People' }
              CardDescription { 'Manage people and their schedules, track medication schedules' }
            end
          end
        end
      end

      def render_medication_icon
        render Icons::Pill.new(size: 24)
      end

      def render_people_icon
        render Icons::Users.new(size: 24)
      end
    end
  end
end
