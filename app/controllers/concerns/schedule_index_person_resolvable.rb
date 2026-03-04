# frozen_string_literal: true

module ScheduleIndexPersonResolvable
  extend ActiveSupport::Concern

  private

  def schedule_index_person
    return Person.new if current_user.nil?
    return current_user.person if current_user.person.nil?

    current_user.person.patients.first || current_user.person
  end
end
