# frozen_string_literal: true

class DashboardController < ApplicationController
  def index
    authorize :dashboard, :index?

    presenter = DashboardPresenter.new(
      current_user: current_user
    )

    render Components::Dashboard::IndexView.new(presenter: presenter)
  end
end
