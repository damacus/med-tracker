# frozen_string_literal: true

# Presenter for the dashboard view that encapsulates data preparation logic
class DashboardPresenter
  attr_reader :current_user

  def initialize(current_user:)
    @current_user = current_user
  end

  def people
    @people ||= load_people
  end

  def active_prescriptions
    @active_prescriptions ||= Prescription.active
                                          .where(person_id: people.select(:id))
                                          .includes(person: :user, medicine: [])
  end

  def upcoming_prescriptions
    @upcoming_prescriptions ||= active_prescriptions.group_by(&:person)
  end

  def doses
    @doses ||= FamilyDashboard::ScheduleQuery.new(people).call
  end

  def next_dose_time
    upcoming = doses.select { |d| d[:status] == :upcoming }
    upcoming.min_by { |d| d[:scheduled_at] }&.dig(:scheduled_at)
  end

  def compliance_percentage
    # Placeholder for actual compliance logic
    # For now, return a realistic-looking mock value
    85
  end

  private

  def load_people
    return Person.none if current_user.nil?

    return all_people if full_access?
    return carer_patients if carer?
    return parent_minor_patients if parent?

    own_person
  end

  def all_people
    Person.includes(:user, prescriptions: :medicine).all
  end

  def carer_patients
    return Person.none if current_user.person.nil?

    current_user.person.patients.includes(:user, prescriptions: :medicine)
  end

  def parent_minor_patients
    return Person.none if current_user.person.nil?

    current_user.person.patients.where(person_type: :minor)
                .includes(:user, prescriptions: :medicine)
  end

  def own_person
    return Person.none if current_user.person.nil?

    Person.where(id: current_user.person.id)
          .includes(:user, prescriptions: :medicine)
  end

  def carer?
    current_user.carer?
  end

  def parent?
    current_user.parent?
  end

  def full_access?
    current_user.administrator? || current_user.doctor? || current_user.nurse?
  end
end
