# frozen_string_literal: true

class DashboardController < ApplicationController
  def index
    authorize :dashboard, :index?

    presenter = DashboardPresenter.new(
      current_user: current_user,
      selected_person_id: params[:dashboard_person_id],
      people_scope: policy_scope(Person),
      household: current_household
    )

    render Components::Dashboard::IndexView.new(presenter: presenter)
  end
end
