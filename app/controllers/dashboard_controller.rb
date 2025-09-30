# frozen_string_literal: true

class DashboardController < ApplicationController
  def index
    people = Person.includes(:user, prescriptions: :medicine).all
    active_prescriptions = Prescription.where(active: true).includes(person: :user, medicine: [])
    upcoming_prescriptions = active_prescriptions.group_by(&:person)

    render Components::Dashboard::IndexView.new(
      people: people,
      active_prescriptions: active_prescriptions,
      upcoming_prescriptions: upcoming_prescriptions,
      url_helpers: self
    )
  end
end
