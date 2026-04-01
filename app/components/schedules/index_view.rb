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
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') { t('schedules.index.title') }
            Text(weight: :muted) { t('schedules.index.subtitle') }
          end

          # ⚡ Bolt Optimization: Use .to_a.any? instead of .any? to materialize the relation
          # into an array in memory. This prevents an extra COUNT/EXISTS query before iterating.
          if schedules.to_a.any?
            div(class: 'rounded-3xl border border-border bg-surface-container-lowest overflow-hidden shadow-sm') do
              table(class: 'w-full text-sm') do
                thead(class: 'bg-surface-container-low text-muted-foreground uppercase tracking-widest text-xs') do
                  tr do
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.person') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.medication') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.dosage') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.frequency') }
                    th(class: 'text-left px-6 py-4') { t('schedules.index.start_date') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.end_date') }
                  end
                end
                tbody(class: 'divide-y divide-border') do
                  schedules.each do |schedule|
                    tr do
                      td(class: 'px-6 py-4 font-semibold text-foreground') do
                        Link(href: person_path(schedule.person), variant: :ghost, class: 'p-0 h-auto no-underline') do
                          schedule.person.name
                        end
                      end
                      td(class: 'px-6 py-4 text-foreground') { schedule.medication.name }
                      td(class: 'px-6 py-4 text-foreground') { dosage_label(schedule) }
                      td(class: 'px-6 py-4 text-foreground') { schedule.frequency }
                      td(class: 'px-6 py-4 text-foreground') { format_date(schedule.start_date) }
                      td(class: 'px-6 py-4 text-foreground') { format_date(schedule.end_date) }
                    end
                  end
                end
              end
            end
          else
            render RubyUI::Card.new(class: 'p-12 text-center rounded-[2rem] border-dashed border-2') do
              Text(weight: :muted) { t('schedules.index.empty') }
            end
          end
        end
      end

      private

      def dosage_label(schedule)
        "#{schedule.dosage.amount} #{schedule.dosage.unit}"
      end

      def format_date(value)
        value ? value.strftime('%Y-%m-%d') : t('schedules.index.ongoing')
      end
    end
  end
end
