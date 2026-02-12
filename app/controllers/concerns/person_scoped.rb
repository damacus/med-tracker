# frozen_string_literal: true

module PersonScoped
  extend ActiveSupport::Concern

  included do
    before_action :set_person
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
  end
end
