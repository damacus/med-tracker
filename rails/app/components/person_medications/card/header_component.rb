# frozen_string_literal: true

module Components
  module PersonMedications
    class Card
      class HeaderComponent < Components::Base
        attr_reader :person_medication

        def initialize(person_medication:)
          @person_medication = person_medication
          super()
        end

        def view_template
          CardHeader(class: 'pb-4 pt-8 px-8') do
            div(class: 'flex justify-between items-start mb-4') do
              render_medication_icon
              div(class: 'flex flex-col items-end gap-2 shrink-0') do
                render_paused_badge if person_medication.paused?
                render Components::Shared::StockBadge.new(medication: person_medication.medication)
              end
            end
            div(class: 'min-w-0') do
              CardTitle(class: 'text-2xl font-black tracking-tight mb-1 text-foreground break-words leading-tight') do
                person_medication.medication.display_name
              end
              CardDescription(class: 'text-on-surface-variant font-bold uppercase text-[10px] tracking-widest') do
                medication_description
              end
            end
          end
        end

        private

        def render_paused_badge
          m3_badge(variant: :outlined, class: 'rounded-full uppercase text-[10px] font-black tracking-widest') do
            t('person_medications.card.paused')
          end
        end

        def medication_description
          parts = []
          dose = DoseAmount.new(person_medication.dose_amount, person_medication.dose_unit).to_s
          parts << dose if dose.present?
          parts << t('people.add_medication.otc_title')
          parts.join(' • ')
        end

        def render_medication_icon
          div(
            class: 'w-12 h-12 rounded-shape-xl bg-secondary-container flex items-center justify-center ' \
                   'text-on-surface-variant ' \
                   'group-hover:text-primary group-hover:bg-primary/5 transition-all'
          ) do
            render Components::Shared::MedicationIcon.new(medication: person_medication.medication, size: 24)
          end
        end
      end
    end
  end
end
