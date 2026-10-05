# frozen_string_literal: true

class DashboardController < ApplicationController
  include TimelineRefreshable

  def index
    authorize :dashboard, :index?

    render dashboard_projection(selected_person_id: selected_person_id, grouping: params[:dashboard_grouping])
  end

  private

  def selected_person_id
    return params[:dashboard_person_id] if params[:dashboard_person_id].present?
    return DashboardPresenter::ALL_FAMILY_PERSON_ID if current_account.dashboard_variant == 'family_lanes'

    params[:dashboard_person_id]
  end
end
