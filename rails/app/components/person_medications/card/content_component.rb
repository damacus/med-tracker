# frozen_string_literal: true

module Components
  module PersonMedications
    class Card
      class ContentComponent < Components::Base
        attr_reader :person_medication

        def initialize(person_medication:)
          @person_medication = person_medication
          super()
        end

        def view_template
          CardContent(class: 'flex-grow space-y-6 px-8') do
            div(class: 'pt-4 border-t border-border space-y-4') do
              render_notes if person_medication.notes.present?
              if person_medication.timing_restrictions?
                render TimingStatusComponent.new(person_medication: person_medication)
              end
            end
          end
        end

        private

        def render_notes
          div(class: 'p-4 bg-primary-container border border-primary/20 rounded-shape-xl') do
            div(class: 'flex items-center gap-2 mb-1') do
              render Icons::FileText.new(size: 14, class: 'text-on-primary-container')
              m3_text(size: '1', weight: 'bold',
                      class: 'font-black uppercase tracking-widest text-on-primary-container') do
                t('person_medications.card.notes')
              end
            end
            m3_text(size: '2', class: 'text-on-primary-container leading-relaxed') { person_medication.notes }
          end
        end
      end
    end
  end
end
