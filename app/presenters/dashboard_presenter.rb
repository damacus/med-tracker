# frozen_string_literal: true

# Presenter for the dashboard view that encapsulates data preparation logic
class DashboardPresenter
  ALL_FAMILY_PERSON_ID = 'all'
  DUE_NOW_STATUSES = %i[available].freeze
  TERMINAL_TASK_STATUSES = %i[taken max_reached].freeze

  delegate :routine_tasks_by_person, :as_needed_by_person, :today_takes_by_person, to: :dashboard_schedule

  attr_reader :current_user, :selected_person_id, :base_people_scope, :household

  def initialize(current_user:, selected_person_id: nil, people_scope: nil, household: nil)
    @current_user = current_user
    @selected_person_id = selected_person_id.presence
    @base_people_scope = people_scope
    @household = household
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
    # ⚡ Bolt Optimization: Eager load schedules and person_medications
    # to prevent N+1 queries when rendering supply item components
    # which calculate estimated_daily_consumption for each medication.
    # Impact: Reduces database queries from O(N) to O(1) for dashboard supply levels.
    @active_schedules ||= active_schedule_scope
                          .where(person_id: people.map(&:id))
                          .includes(person: :user, medication: [:schedules, :person_medications])
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

  def next_dose_time = next_due_time

  def next_due_value
    return I18n.t('dashboard.stats.now') if due_now_count.positive?

    time = next_due_time
    return time.strftime('%H:%M') if time

    I18n.t('dashboard.stats.no_upcoming_doses')
  end

  def due_now_count
    action_rows.count { |row| due_now_row?(row) }
  end

  def tasks_left_count
    action_rows.count { |row| task_left_row?(row) }
  end

  def smart_insights
    @smart_insights ||= SmartInsights::IndexQuery.new(
      people: people,
      start_date: Time.zone.today - 6.days,
      end_date: Time.zone.today
    ).call
  end

  def can_view_reports?
    ReportPolicy.new(policy_context, :report).index?
  end

  private

  def next_due_time
    action_rows.filter_map { |row| next_due_time_for(row) }.min
  end

  def action_rows
    @action_rows ||= routine_tasks_by_person.values.flatten + as_needed_by_person.values.flatten
  end

  def due_now_row?(row)
    return true if DUE_NOW_STATUSES.include?(row[:status])
    return false unless row[:status] == :upcoming

    row[:scheduled_at].blank? || row[:scheduled_at] <= Time.current
  end

  def task_left_row?(row)
    TERMINAL_TASK_STATUSES.exclude?(row[:status])
  end

  def next_due_time_for(row)
    return if TERMINAL_TASK_STATUSES.include?(row[:status])
    return if due_now_row?(row)

    row[:scheduled_at]
  end

  def load_people
    return Person.none if current_user.nil?
    return Person.none unless base_people_scope

    people_scope(base_people_scope)
  end

  def active_schedule_scope
    scope = Schedule.active
    return scope unless household

    scope.where(household: household)
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
    selectable_people.find { |person| person == current_person } ||
      selectable_people.min_by { |person| [person.name.to_s.downcase, person.id] }
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

  def policy_context
    AuthorizationContext.current || current_user
  end

  def dashboard_schedule
    @dashboard_schedule ||= FamilyDashboard::ScheduleQuery.new(people, current_user: current_user).tap(&:call)
  end
end
