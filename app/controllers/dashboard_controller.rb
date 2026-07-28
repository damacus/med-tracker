# frozen_string_literal: true

class DashboardController < ApplicationController
  def index
    authorize :dashboard, :index?

    presenter = DashboardPresenter.new(
      current_user: current_user,
      selected_person_id: selected_person_id,
      people_scope: policy_scope(Person),
      household: current_household
    )

    render dashboard_view(presenter)
  end

  private

  def selected_person_id
    return params[:dashboard_person_id] if params[:dashboard_person_id].present?
    return DashboardPresenter::ALL_FAMILY_PERSON_ID if current_account.dashboard_variant == 'family_lanes'

    params[:dashboard_person_id]
  end

  def dashboard_view(presenter)
    case current_account.dashboard_variant
    when 'time_first'
      Components::Dashboard::TimeFirstView.new(presenter: presenter)
    when 'family_lanes'
      Components::Dashboard::FamilyLanesView.new(presenter: presenter, grouping: params[:dashboard_grouping])
    when 'calm_focus'
      Components::Dashboard::CalmFocusView.new(presenter: presenter)
    else
      Components::Dashboard::IndexView.new(presenter: presenter)
    end
  end
end
