# frozen_string_literal: true

module Components
  module Schedules
    class Card
      class HeaderComponent < Components::Base
        attr_reader :schedule, :presenter

        def initialize(schedule:, presenter:)
          @schedule = schedule
          @presenter = presenter
          super()
        end

        def view_template
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medication_icon
              div(class: 'flex flex-col items-end gap-2 shrink-0') do
                render Components::Shared::StockBadge.new(medication: schedule.medication)
                status_badge
              end
            end
            div(class: 'min-w-0') do
              m3_heading(
                variant: :title_large,
                level: 3,
                class: 'font-black tracking-tight mb-1 text-foreground break-words leading-tight'
              ) do
                schedule.medication.display_name
              end
              m3_text(variant: :label_small, class: 'text-on-surface-variant font-black uppercase tracking-widest') do
                presenter.dose_description
              end
            end
          end
        end

        private

        def render_medication_icon
          div(
            class: 'w-12 h-12 rounded-2xl bg-secondary-container flex items-center ' \
                   'justify-center text-on-surface-variant ' \
                   'group-hover:text-primary group-hover:bg-primary/5 transition-all'
          ) do
            render Icons::Pill.new(size: 24)
          end
        end

        def status_badge
          return unless presenter.status_badge?

          m3_badge(
            variant: presenter.status_badge_variant,
            class: 'px-3 py-1 text-[10px] font-black uppercase tracking-wider'
          ) do
            plain presenter.status_badge_label
          end
        end
      end
    end
  end
end
