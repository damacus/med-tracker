# frozen_string_literal: true

class DashboardController < ApplicationController
  include Pundit::Authorization

  def index
    authorize :dashboard, :index?

    # Fetch family doses using our new query object
    query = FamilyDashboard::ScheduleQuery.new(current_user.person)
    @doses = query.call

    render Components::Dashboard::FamilySummary.new(doses: @doses)
  end
end
