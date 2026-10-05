# frozen_string_literal: true

require 'benchmark'
require 'fileutils'

profile_email = ENV.fetch('PROFILE_DASHBOARD_EMAIL', 'admin@example.com')
selected_person_id = ENV.fetch('PROFILE_DASHBOARD_PERSON_ID', DashboardPresenter::ALL_FAMILY_PERSON_ID)
artifact_path = ENV.fetch('PROFILE_DASHBOARD_ARTIFACT', 'docs/performance/profiles/dashboard-baseline.vernier.json.gz')
artifact_full_path = Rails.root.join(artifact_path)
summary_path = Rails.root.join(ENV.fetch('PROFILE_DASHBOARD_SUMMARY', 'docs/performance/dashboard-baseline.md'))
FileUtils.mkdir_p(artifact_full_path.dirname)
FileUtils.mkdir_p(summary_path.dirname)

account = Account.find_by!(email: profile_email)
user = User.find_by!(email_address: profile_email)
membership = account.first_active_household_membership
raise ActiveRecord::RecordNotFound, "No active household membership for #{profile_email}" unless membership

household = membership.household
result = nil

elapsed = Benchmark.realtime do
  TenantContext.with(account: account, household: household, membership: membership, request_id: 'profile-dashboard') do
    context = AuthorizationContext.current
    people_scope = PersonPolicy::Scope.new(context, Person.all).resolve
    presenter = DashboardPresenter.new(
      current_user: user,
      selected_person_id: selected_person_id,
      people_scope: people_scope,
      household: household
    )

    result = {
      people: presenter.people.size,
      options: presenter.dashboard_person_options.size,
      routine_tasks: presenter.routine_tasks_by_person.values.sum(&:size),
      as_needed_tasks: presenter.as_needed_by_person.values.sum(&:size),
      today_takes: presenter.today_takes_by_person.values.sum(&:size),
      due_now: presenter.due_now_count,
      tasks_left: presenter.tasks_left_count,
      next_due: presenter.next_due_value,
      reports_visible: presenter.can_view_reports?
    }
  end
end

summary = <<~MARKDOWN
  # Dashboard Vernier Baseline

  - Captured at: #{Time.current.utc.iso8601}
  - Account: #{profile_email}
  - Household: #{household.slug}
  - Selected person: #{selected_person_id}
  - Vernier artifact: #{artifact_path}
  - Elapsed wall time: #{(elapsed * 1000).round(2)}ms
  - People: #{result.fetch(:people)}
  - Selector options: #{result.fetch(:options)}
  - Routine tasks: #{result.fetch(:routine_tasks)}
  - As-needed tasks: #{result.fetch(:as_needed_tasks)}
  - Today's takes: #{result.fetch(:today_takes)}
  - Due now: #{result.fetch(:due_now)}
  - Tasks left: #{result.fetch(:tasks_left)}
  - Next due: #{result.fetch(:next_due)}
  - Reports visible: #{result.fetch(:reports_visible)}
MARKDOWN

File.write(summary_path, summary)
puts summary
