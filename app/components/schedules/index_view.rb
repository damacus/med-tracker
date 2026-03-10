# frozen_string_literal: true

module Components
  module Schedules
    class IndexView < Components::Base
      attr_reader :schedules

      def initialize(schedules:)
        @schedules = schedules
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-6xl', data: { testid: 'active-schedules-list' }) do
          div(class: 'mb-8') do
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { 'Active Schedules' }
            Text(weight: :muted) { 'Schedules currently active for people you can access.' }
          end

          # ⚡ Bolt Optimization: Use .to_a.any? instead of .any? to materialize the relation
          # into an array in memory. This prevents an extra COUNT/EXISTS query before iterating.
          if schedules.to_a.any?
            div(class: 'rounded-3xl border border-slate-100 bg-white overflow-hidden shadow-sm') do
              table(class: 'w-full text-sm') do
                thead(class: 'bg-slate-50 text-slate-500 uppercase tracking-widest text-xs') do
                  tr do
                    th(class: 'text-left px-6 py-4') { 'Person' }
                    th(class: 'text-left px-6 py-4') { 'Medication' }
                    th(class: 'text-left px-6 py-4') { 'Dosage' }
                    th(class: 'text-left px-6 py-4') { 'Frequency' }
                    th(class: 'text-left px-6 py-4') { 'Start Date' }
                    th(class: 'text-left px-6 py-4') { 'End Date' }
                  end
                end
                tbody(class: 'divide-y divide-slate-100') do
                  schedules.each do |schedule|
                    tr do
                      td(class: 'px-6 py-4 font-semibold text-slate-900') do
                        Link(href: person_path(schedule.person), variant: :ghost, class: 'p-0 h-auto no-underline') do
                          schedule.person.name
                        end
                      end
                      td(class: 'px-6 py-4 text-slate-700') { schedule.medication.name }
                      td(class: 'px-6 py-4 text-slate-700') { dosage_label(schedule) }
                      td(class: 'px-6 py-4 text-slate-700') { schedule.frequency }
                      td(class: 'px-6 py-4 text-slate-700') { format_date(schedule.start_date) }
                      td(class: 'px-6 py-4 text-slate-700') { format_date(schedule.end_date) }
                    end
                  end
                end
              end
            end
          else
            render RubyUI::Card.new(class: 'p-12 text-center rounded-[2rem] border-dashed border-2') do
              Text(weight: :muted) { 'No active schedules found.' }
            end
          end
        end
      end

      private

      def dosage_label(schedule)
        "#{schedule.dosage.amount} #{schedule.dosage.unit}"
      end

      def format_date(value)
        value ? value.strftime('%Y-%m-%d') : 'Ongoing'
      end
    end
  end
end
