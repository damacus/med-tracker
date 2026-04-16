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
        div(class: 'container mx-auto px-4 py-12 max-w-6xl', data: { testid: 'active-schedules-list' }) do
          div(class: 'mb-10 space-y-2') do
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') { t('schedules.index.title') }
            m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') { t('schedules.index.subtitle') }
          end

          # ⚡ Bolt Optimization: Use .to_a.any? instead of .any? to materialize the relation
          # into an array in memory. This prevents an extra COUNT/EXISTS query before iterating.
          if schedules.to_a.any?
            div(class: 'rounded-shape-xl border border-outline-variant/30 bg-surface-container-lowest overflow-hidden shadow-elevation-1') do
              table(class: 'w-full text-sm') do
                thead(class: 'bg-surface-container-low text-on-surface-variant uppercase tracking-widest text-[10px] font-black') do
                  tr do
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.person') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.medication') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.dosage') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.frequency') }
                    th(class: 'text-left px-6 py-4') { t('schedules.index.start_date') }
                    th(class: 'text-left px-6 py-4') { t('dashboard.table.end_date') }
                  end
                end
                tbody(class: 'divide-y divide-outline-variant/30') do
                  schedules.each do |schedule|
                    tr do
                      td(class: 'px-6 py-5 font-bold text-foreground') do
                        m3_link(href: person_path(schedule.person), variant: :text, size: :sm, class: 'p-0 h-auto no-underline hover:text-primary') do
                          schedule.person.name
                        end
                      end
                      td(class: 'px-6 py-5 text-foreground font-medium') { schedule.medication.name }
                      td(class: 'px-6 py-5 text-foreground font-medium') { dosage_label(schedule) }
                      td(class: 'px-6 py-5 text-foreground font-medium') { schedule.frequency }
                      td(class: 'px-6 py-5 text-on-surface-variant font-medium') { format_date(schedule.start_date) }
                      td(class: 'px-6 py-5 text-on-surface-variant font-medium') { format_date(schedule.end_date) }
                    end
                  end
                end
              end
            end
          else
            m3_card(variant: :elevated, class: 'p-16 text-center rounded-[2.5rem] border-dashed border-2 border-outline-variant/50') do
              m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium italic') { t('schedules.index.empty') }
            end
          end
        end
      end

      private

      def dosage_label(schedule)
        "#{schedule.dose_amount} #{schedule.dose_unit}"
      end

      def format_date(value)
        value ? value.strftime('%Y-%m-%d') : t('schedules.index.ongoing')
      end
    end
  end
end