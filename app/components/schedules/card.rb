# frozen_string_literal: true

module Components
  module Schedules
    # Renders a schedule card with medication details and take medication form
    class Card < Components::Base
      attr_reader :schedule, :person, :current_user

      def initialize(schedule:, person:, current_user: nil)
        @schedule = schedule
        @person = person
        @current_user = current_user
        super()
      end

      def view_template
        render M3::Card.new(
          id: tenant_dom_id(schedule),
          class: card_class
        ) do
          render HeaderComponent.new(schedule: schedule, presenter: presenter)
          render DoseStatusComponent.new(schedule: schedule, presenter: presenter)
          render ActionsComponent.new(
            schedule: schedule,
            person: person,
            presenter: presenter,
            current_user: current_user
          )
        end
      end

      private

      def presenter
        @presenter ||= ::Schedules::CardPresenter.new(
          schedule: schedule,
          current_user: current_user,
          person: person
        )
      end

      def card_class
        base = 'h-full flex flex-col border-none shadow-[0_15px_40px_rgba(0,0,0,0.08)] bg-card ' \
               'rounded-[2rem] transition-all duration-300 group overflow-hidden'
        return "#{base} opacity-70 grayscale-[0.2]" if schedule.paused?

        "#{base} hover:scale-[1.02] hover:shadow-2xl"
      end
    end
  end
end
