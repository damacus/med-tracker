# frozen_string_literal: true

# Presenter for the dashboard view that encapsulates data preparation logic
class DashboardPresenter
  ALL_FAMILY_PERSON_ID = 'all'

  delegate :routine_tasks_by_person, :as_needed_by_person, :dashboard_people, to: :dashboard_schedule

  attr_reader :current_user, :selected_person_id

  def initialize(current_user:, selected_person_id: nil)
    @current_user = current_user
    @selected_person_id = selected_person_id.presence
  end

  def people
    @people ||= scoped_people
  end

  def selectable_people
    @selectable_people ||= load_people.to_a
  end

  def selected_person
    return if all_family_selected?

    @selected_person ||= requested_person || default_selected_person
  end

  def all_family_selected?
    selected_person_id == ALL_FAMILY_PERSON_ID
  end

  def dashboard_person_options
    selectable_people.map do |person|
      {
        id: person.id.to_s,
        label: person.name,
        initials: initials_for(person),
        person: person,
        selected: selected_person == person,
        all_family: false
      }
    end + [all_family_option]
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

  def load_people
    return Person.none if current_user.nil?

    return people_scope(Person.all) if full_access?

    current_person_scope
  end

  def scoped_people
    return selectable_people if all_family_selected?
    return [] unless selected_person

    [selected_person]
  end

  def requested_person
    return if selected_person_id.blank?

    selectable_people.find { |person| person.id.to_s == selected_person_id.to_s }
  end

  def default_selected_person
    selectable_people.find { |person| person == current_person } || selectable_people.first
  end

  def initials_for(person)
    person.name.to_s.split.map(&:first).join.upcase.presence || '?'
  end

  def all_family_option
    {
      id: ALL_FAMILY_PERSON_ID,
      label: I18n.t('dashboard.person_selector.all_family'),
      initials: I18n.t('dashboard.person_selector.all_family_initials'),
      person: nil,
      selected: all_family_selected?,
      all_family: true
    }
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
