# frozen_string_literal: true

# Presenter for the dashboard view that encapsulates data preparation logic
class DashboardPresenter
  delegate :routine_tasks_by_person, :as_needed_by_person, to: :dashboard_schedule

  attr_reader :current_user

  def initialize(current_user:)
    @current_user = current_user
  end

  def people
    @people ||= load_people.to_a
  end

  def active_schedules
    @active_schedules ||= Schedule.active
                                  .where(person_id: people.map(&:id))
                                  .includes(person: :user, medication: [])
                                  .to_a
  end

  def upcoming_schedules
    @upcoming_schedules ||= active_schedules.group_by(&:person)
  end

  def doses
    dashboard_schedule.routine_tasks
  end

  def routine_tasks_due?
    doses.any? { |d| d[:status] == :upcoming }
  end

  def next_dose_time
    upcoming = doses.select { |d| d[:status] == :upcoming }
    upcoming.filter_map { |d| d[:scheduled_at] }.min
  end

  def compliance_percentage
    expected, actual = compliance_counts
    return 100 if expected.zero?

    [(actual.to_f / expected * 100).round, 100].min
  end

  def smart_insights
    @smart_insights ||= SmartInsights::IndexQuery.new(
      people: people,
      start_date: Time.zone.today - 6.days,
      end_date: Time.zone.today
    ).call
  end

  def can_view_reports?
    ReportPolicy.new(current_user, :report).index?
  end

  private

  def compliance_counts
    daily_data = Reports::IndexQuery.new(
      people: people,
      start_date: Time.zone.today - 6.days,
      end_date: Time.zone.today
    ).call.daily_data

    [
      daily_data.sum { |day| day[:expected] },
      daily_data.sum { |day| day[:actual] }
    ]
  end

  def load_people
    return Person.none if current_user.nil?

    return people_scope(Person.all) if full_access?

    current_person_scope
  end

  def people_scope(scope_or_ids)
    scope = if scope_or_ids.is_a?(ActiveRecord::Relation)
              scope_or_ids
            else
              Person.where(id: Array(scope_or_ids).uniq)
            end

    scope.includes(:user, schedules: [:medication], person_medications: :medication)
  end

  def current_person
    current_user.person
  end

  def current_person_scope
    return Person.none if current_person.nil?
    return people_scope(current_person.patients) if carer?
    return people_scope(parent_people_ids) if parent?

    people_scope(current_person.id)
  end

  def parent_people_ids
    [current_person.id] + current_person.patients.where(person_type: :minor).pluck(:id)
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

  def dashboard_schedule
    @dashboard_schedule ||= FamilyDashboard::ScheduleQuery.new(people, current_user: current_user).tap(&:call)
  end
end
