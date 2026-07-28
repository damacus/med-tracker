# frozen_string_literal: true

require 'fileutils'
require Rails.root.join('lib/performance/dashboard_request_profiler')

profile_email = ENV.fetch('PROFILE_DASHBOARD_EMAIL', 'admin@example.com')
profile_password = ENV.fetch('PROFILE_DASHBOARD_PASSWORD', 'password')
selected_person_id = ENV.fetch('PROFILE_DASHBOARD_PERSON_ID', DashboardPresenter::ALL_FAMILY_PERSON_ID)
artifact_path = ENV.fetch('PROFILE_DASHBOARD_ARTIFACT', 'docs/performance/profiles/dashboard-baseline.vernier.json.gz')
artifact_full_path = Rails.root.join(artifact_path)
summary_path = Rails.root.join(ENV.fetch('PROFILE_DASHBOARD_SUMMARY', 'docs/performance/dashboard-baseline.md'))
warmup_iterations = Integer(
  ENV.fetch(
    'PROFILE_DASHBOARD_WARMUP_ITERATIONS',
    Performance::DashboardRequestProfiler::DEFAULT_WARMUP_ITERATIONS.to_s
  ),
  10
)
measured_iterations = Integer(
  ENV.fetch(
    'PROFILE_DASHBOARD_MEASURED_ITERATIONS',
    Performance::DashboardRequestProfiler::DEFAULT_MEASURED_ITERATIONS.to_s
  ),
  10
)
FileUtils.mkdir_p(artifact_full_path.dirname)
FileUtils.mkdir_p(summary_path.dirname)

configuration = Performance::DashboardRequestProfiler::Configuration.new(
  application: Rails.application,
  profile_email:,
  password: profile_password,
  selected_person_id:,
  artifact_path:,
  warmup_iterations:,
  measured_iterations:,
  host: 'localhost'
)
result = Performance::DashboardRequestProfiler.new(configuration).run
summary = result.to_markdown

File.write(summary_path, summary)
puts summary
