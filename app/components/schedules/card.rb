# frozen_string_literal: true

module Components
  module Schedules
    # Renders a schedule card with medication details and take medication form
    class Card < Components::Base
      attr_reader :schedule, :person, :todays_takes, :current_user

      def initialize(schedule:, person:, todays_takes: nil, current_user: nil)
        @schedule = schedule
        @person = person
        @todays_takes = todays_takes
        @current_user = current_user
        super()
      end

      def view_template
        render(
          M3::Card.new(
            id: "schedule_#{schedule.id}",
            class: "h-full flex flex-col border-none border-t-4 #{presenter.status_top_border_class} " \
              "shadow-[0_15px_40px_rgba(0,0,0,0.08)] bg-card rounded-[2rem] transition-all " \
              "duration-300 hover:scale-[1.02] hover:shadow-2xl group overflow-hidden"
          )
        ) do
          render(HeaderComponent.new(schedule: schedule, presenter: presenter))
          render(DoseStatusComponent.new(schedule: schedule, presenter: presenter))
          render(
            ActionsComponent.new(
              schedule: schedule,
              person: person,
              presenter: presenter,
              current_user: current_user
            )
          )
        end
      end

      private

      def presenter
        @presenter ||= ::Schedules::CardPresenter.new(
          schedule: schedule,
          todays_takes: todays_takes,
          current_user: current_user,
          person: person
        )
      end
    end
  end
end
