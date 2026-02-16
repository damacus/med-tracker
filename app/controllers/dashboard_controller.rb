# frozen_string_literal: true

class DashboardController < ApplicationController
  include Pundit::Authorization

  def index
    authorize :dashboard, :index?

    # Core data for stats and existing logic
    @people = scoped_people
    @active_prescriptions = scoped_prescriptions
    @upcoming_prescriptions = @active_prescriptions.group_by(&:person)

    # Chronological family doses for our new timeline
    query = FamilyDashboard::ScheduleQuery.new(@people)
    @doses = query.call

    render Components::Dashboard::IndexView.new(
      people: @people,
      active_prescriptions: @active_prescriptions,
      upcoming_prescriptions: @upcoming_prescriptions,
      doses: @doses,
      url_helpers: self,
      current_user: current_user
    )
  end

  private

  def scoped_people
    if full_access?
      Person.includes(:user, prescriptions: :medicine).all
    elsif current_user.carer?
      current_user.person.patients.includes(:user, prescriptions: :medicine)
    elsif current_user.parent?
      current_user.person.patients.where(person_type: :minor)
                  .includes(:user, prescriptions: :medicine)
    else
      Person.where(id: current_user.person.id)
            .includes(:user, prescriptions: :medicine)
    end
  end

  def scoped_prescriptions
    person_ids = scoped_people.pluck(:id)
    Prescription.where(active: true, person_id: person_ids)
                .includes(person: :user, medicine: [])
  end

  def full_access?
    current_user.administrator? || current_user.doctor? || current_user.nurse?
  end
end
