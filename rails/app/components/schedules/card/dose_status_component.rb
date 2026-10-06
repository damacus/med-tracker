# frozen_string_literal: true

module Components
  module Schedules
    class Card
      class DoseStatusComponent < Components::Base
        attr_reader :schedule, :presenter

        def initialize(schedule:, presenter:)
          @schedule = schedule
          @presenter = presenter
          super()
        end

        def view_template
          CardContent(class: 'flex-grow space-y-6 px-8') do
            div(class: 'pt-4 border-t border-border space-y-6') do
              render_date_details
              render_notes if schedule.notes.present?
            end
          end
        end

        private

        def render_date_details
          div(class: 'grid grid-cols-2 gap-4') do
            div(class: 'space-y-1') do
              m3_text(variant: :label_small, class: date_label_class) do
                t('schedules.card.started')
              end
              m3_text(variant: :body_medium, class: 'text-on-surface-variant font-bold block') do
                schedule.start_date.strftime('%b %d, %Y')
              end
            end

            if schedule.end_date
              div(class: 'space-y-1') do
                m3_text(variant: :label_small, class: date_label_class) do
                  t('schedules.card.ends')
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant font-bold block') do
                  schedule.end_date.strftime('%b %d, %Y')
                end
              end
            end
          end
        end

        def date_label_class
          'uppercase tracking-widest text-on-surface-variant/50 font-black text-[10px]'
        end

        def render_notes
          div(class: 'space-y-2') do
            m3_text(variant: :label_small, class: date_label_class) do
              t('schedules.card.notes')
            end
            m3_text(variant: :body_medium, class: 'text-on-surface-variant leading-relaxed font-medium block') do
              schedule.notes
            end
          end
        end
      end
    end
  end
end
