# frozen_string_literal: true

module Schedules
  class CardPresenter
    attr_reader :schedule, :current_user, :person

    def initialize(schedule:, current_user:, person:)
      @schedule = schedule
      @current_user = current_user
      @person = person
    end

    def dose_description
      "#{dose_text} • #{schedule.frequency}"
    end

    private

    def dose_text
      DoseAmount.new(schedule.dose_amount, schedule.dose_unit).to_s
    end

    def own_dose?
      return true if current_user.nil?

      current_user.person == person
    end
  end
end
