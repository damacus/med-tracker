# frozen_string_literal: true

module Components
  module PersonMedications
    class Card
      class TimingStatusComponent < Components::Base
        attr_reader :person_medication

        def initialize(person_medication:)
          @person_medication = person_medication
          super()
        end

        def view_template
          div(class: 'p-4 bg-secondary-container border border-border rounded-shape-xl') do
            div(class: 'flex items-center gap-2 mb-2') do
              render Icons::Clock.new(size: 14, class: 'text-on-surface-variant')
              m3_text(
                size: '1',
                weight: 'bold',
                class: 'font-black uppercase tracking-widest text-on-surface-variant'
              ) do
                t('person_medications.card.timing_restrictions')
              end
            end
            ul(class: 'space-y-1.5') do
              render_max_daily_doses if person_medication.max_daily_doses.present?
              render_min_hours_between_doses if person_medication.min_hours_between_doses.present?
            end
          end
        end

        private

        def render_max_daily_doses
          li(class: 'flex items-center gap-2') do
            div(class: 'w-1 h-1 rounded-full bg-secondary-container-foreground')
            m3_text(size: '2', weight: 'semibold', class: 'text-on-surface-variant') do
              t('person_medications.card.max_doses_per_day', count: person_medication.max_daily_doses)
            end
          end
        end

        def render_min_hours_between_doses
          li(class: 'flex items-center gap-2') do
            div(class: 'w-1 h-1 rounded-full bg-secondary-container-foreground')
            m3_text(size: '2', weight: 'semibold', class: 'text-on-surface-variant') do
              t('person_medications.card.wait_hours', hours: person_medication.min_hours_between_doses)
            end
          end
        end
      end
    end
  end
end
