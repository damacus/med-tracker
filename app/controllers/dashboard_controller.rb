# frozen_string_literal: true

class DashboardController < ApplicationController
  def index
    users = User.includes(prescriptions: :medicine).all
    active_prescriptions = Prescription.where(active: true).includes(:user, :medicine)
    upcoming_prescriptions = active_prescriptions.group_by(&:user)

    render Components::Dashboard::IndexView.new(
      users: users,
      active_prescriptions: active_prescriptions,
      upcoming_prescriptions: upcoming_prescriptions,
      url_helpers: self
    )
  end
end
