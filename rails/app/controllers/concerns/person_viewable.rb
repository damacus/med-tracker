# frozen_string_literal: true

module PersonViewable
  extend ActiveSupport::Concern

  private

  def person_show_view(person)
    show_data = PersonShowQuery.new(person: person).call

    Components::People::ShowView.new(
      person: person,
      schedules: show_data.schedules,
      person_medications: show_data.person_medications,
      current_user: current_user
    )
  end
end
