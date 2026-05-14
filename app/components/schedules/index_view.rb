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
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') do
              t('schedules.index.title')
            end
            m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
              t('schedules.index.subtitle')
            end
          end

          # ⚡ Bolt Optimization: Use .to_a.any? instead of .any? to materialize the relation
          # into an array in memory. This prevents an extra COUNT/EXISTS query before iterating.
          if schedules.to_a.any?
            render_mobile_schedule_cards
            render_desktop_schedule_table
          else
            m3_card(variant: :elevated,
                    class: 'p-16 text-center rounded-[2.5rem] border-dashed border-2 border-outline-variant/50') do
              m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium italic') do
                t('schedules.index.empty')
              end
            end
          end
        end
      end

      private

      def render_mobile_schedule_cards
        div(class: 'space-y-4 md:hidden', data: { testid: 'schedules-mobile-list' }) do
          schedules.each do |schedule|
            m3_card(class: 'rounded-[2rem] border border-outline-variant/40 bg-card p-5 shadow-elevation-1') do
              div(class: 'space-y-4') do
                div(class: 'flex items-start justify-between gap-3') do
                  div(class: 'min-w-0') do
                    m3_text(size: '2', weight: 'muted',
                            class: 'uppercase tracking-widest font-bold') { t('dashboard.table.person') }
                    m3_link(href: person_path(schedule.person), variant: :text,
                            class: 'mt-1 h-auto p-0 text-base font-bold no-underline') do
                      schedule.person.name
                    end
                  end
                  span(class: 'shrink-0 rounded-full bg-primary/10 px-3 py-1 text-xs font-bold text-primary') do
                    dosage_label(schedule)
                  end
                end

                div(class: 'min-w-0') do
                  m3_text(size: '2', weight: 'muted',
                          class: 'uppercase tracking-widest font-bold') { t('dashboard.table.medication') }
                  m3_text(class: 'mt-1 break-words font-semibold text-foreground') do
                    schedule.medication.display_name
                  end
                end

                dl(class: 'grid grid-cols-2 gap-3 border-t border-outline-variant/30 pt-4 text-sm') do
                  render_mobile_detail(t('dashboard.table.frequency'), schedule.frequency)
                  render_mobile_detail(t('schedules.index.start_date'), format_date(schedule.start_date))
                  render_mobile_detail(t('dashboard.table.end_date'), format_date(schedule.end_date))
                end
              end
            end
          end
        end
      end

      def render_desktop_schedule_table
        div(data: { testid: 'schedules-desktop-table' }, class: 'hidden md:block') do
          div(
            class: 'rounded-shape-xl border border-outline-variant/30 bg-surface-container-lowest ' \
                   'overflow-hidden shadow-elevation-1'
          ) do
            table(class: 'w-full text-sm') do
              thead(
                class: 'bg-surface-container-low text-on-surface-variant uppercase tracking-widest ' \
                       'text-[10px] font-black'
              ) do
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
                      m3_link(href: person_path(schedule.person), variant: :text, size: :sm,
                              class: 'p-0 h-auto no-underline hover:text-primary') do
                        schedule.person.name
                      end
                    end
                    td(class: 'px-6 py-5 text-foreground font-medium') { schedule.medication.display_name }
                    td(class: 'px-6 py-5 text-foreground font-medium') { dosage_label(schedule) }
                    td(class: 'px-6 py-5 text-foreground font-medium') { schedule.frequency }
                    td(class: 'px-6 py-5 text-on-surface-variant font-medium') { format_date(schedule.start_date) }
                    td(class: 'px-6 py-5 text-on-surface-variant font-medium') { format_date(schedule.end_date) }
                  end
                end
              end
            end
          end
        end
      end

      def render_mobile_detail(label, value)
        div do
          dt(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { label }
          dd(class: 'mt-1 break-words font-semibold text-foreground') { value }
        end
      end

      def dosage_label(schedule)
        DoseAmount.new(schedule.dose_amount, schedule.dose_unit).label
      end

      def format_date(value)
        value ? value.strftime('%Y-%m-%d') : t('schedules.index.ongoing')
      end
    end
  end
end
