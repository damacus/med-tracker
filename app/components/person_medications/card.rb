# frozen_string_literal: true

module Components
  module PersonMedications
    # Renders a person medication card with take medication functionality
    class Card < Components::Base
      attr_reader :person_medication, :person, :current_user

      def initialize(person_medication:, person:, current_user: nil)
        @person_medication = person_medication
        @person = person
        @current_user = current_user
        super()
      end

      def view_template
        render M3::Card.new(
          id: tenant_dom_id(person_medication),
          class: card_class
        ) do
          render HeaderComponent.new(person_medication: person_medication)
          render ContentComponent.new(person_medication: person_medication)
          render CardFooter(class: 'px-6 pb-8 pt-2') do
            render ActionsComponent.new(
              person_medication: person_medication,
              person: person,
              current_user: current_user
            )
          end
        end
      end

      private

      def card_class
        base = 'h-full flex flex-col border-none border-l-4 border-l-primary ' \
               'shadow-[0_8px_30px_rgb(0,0,0,0.06)] bg-card rounded-[2.5rem] transition-all ' \
               'duration-300 group overflow-hidden'
        return "#{base} opacity-70 grayscale-[0.2]" if person_medication.paused?

        "#{base} hover:scale-[1.02] hover:shadow-xl"
      end
    end
  end
end
