# frozen_string_literal: true

module Admin
  class PeopleController < BaseController
    def index
      authorize :admin_people, :index?

      people = Person.needing_carer_assignment.order(:name)
      render Components::Admin::People::IndexView.new(people: people)
    end
  end
end
