# frozen_string_literal: true

require 'action_dispatch/testing/integration'
require 'active_support/notifications'
require 'nokogiri'

module Performance
  class DashboardRequestProfiler
    DEFAULT_WARMUP_ITERATIONS = 2
    DEFAULT_MEASURED_ITERATIONS = 10
    SQL_EVENT = 'sql.active_record'
    IGNORED_SQL_NAMES = %w[SCHEMA TRANSACTION].freeze
    RESULT_CONFIGURATION_KEYS = %i[
      profile_email selected_person_id artifact_path warmup_iterations measured_iterations
    ].freeze

    Configuration = Data.define(
      :application,
      :profile_email,
      :password,
      :selected_person_id,
      :artifact_path,
      :warmup_iterations,
      :measured_iterations,
      :host
    )
    Measurement = Data.define(:elapsed_ms, :sql_count, :allocations)
    MeasurementTools = Data.define(:clock, :allocation_counter)
    DEFAULT_MEASUREMENT_TOOLS = MeasurementTools.new(
      clock: -> { Process.clock_gettime(Process::CLOCK_MONOTONIC) },
      allocation_counter: -> { GC.stat(:total_allocated_objects) }
    )

    Result = Data.define(
      :captured_at,
      :profile_email,
      :household_slug,
      :selected_person_id,
      :artifact_path,
      :request_path,
      :warmup_iterations,
      :measured_iterations,
      :response_status,
      :measurements
    ) do
      def to_markdown
        "#{(['# Dashboard Request Profile', ''] + configuration_lines + percentile_lines).join("\n")}\n"
      end

      private

      def configuration_lines
        [
          "- Captured at: #{captured_at.utc.iso8601}",
          "- Account: #{profile_email}",
          "- Household: #{household_slug}",
          "- Selected person: #{selected_person_id}",
          "- Request path: #{request_path}",
          "- Vernier artifact: #{artifact_path}",
          "- Warmup iterations: #{warmup_iterations}",
          "- Measured iterations: #{measured_iterations}",
          "- HTTP status: #{response_status}"
        ]
      end

      def percentile_lines
        [
          "- Request latency p50: #{format('%.2f', percentile(:elapsed_ms, 0.50))}ms",
          "- Request latency p95: #{format('%.2f', percentile(:elapsed_ms, 0.95))}ms",
          "- SQL queries p50: #{percentile(:sql_count, 0.50)}",
          "- SQL queries p95: #{percentile(:sql_count, 0.95)}",
          "- Allocations p50: #{percentile(:allocations, 0.50)}",
          "- Allocations p95: #{percentile(:allocations, 0.95)}"
        ]
      end

      def percentile(attribute, quantile)
        values = measurements.map { |measurement| measurement.public_send(attribute) }.sort
        values.fetch((values.size * quantile).ceil - 1)
      end
    end

    def initialize(configuration, measurement_tools: DEFAULT_MEASUREMENT_TOOLS)
      @configuration = configuration
      @measurement_tools = measurement_tools
      validate_iterations
    end

    def run
      account = Account.find_by!(email: profile_email)
      membership = account.first_active_household_membership
      raise ActiveRecord::RecordNotFound, "No active household membership for #{profile_email}" unless membership

      session = authenticated_session
      path = request_path(membership.household)
      build_result(membership, session, path, capture_measurements(session, path))
    end

    private

    attr_reader :configuration, :measurement_tools

    delegate :application, :host, :measured_iterations, :password, :profile_email, :selected_person_id,
             :warmup_iterations, to: :configuration
    delegate :allocation_counter, :clock, to: :measurement_tools

    def validate_iterations
      raise ArgumentError, 'warmup_iterations must be at least 0' if warmup_iterations.negative?
      raise ArgumentError, 'measured_iterations must be at least 1' unless measured_iterations.positive?
    end

    def authenticated_session
      ActionDispatch::Integration::Session.new(application).tap do |session|
        session.host! host
        authenticate(session)
      end
    end

    def build_result(membership, session, path, measurements)
      Result.new(
        **configuration.to_h.slice(*RESULT_CONFIGURATION_KEYS),
        captured_at: Time.current,
        household_slug: membership.household.slug,
        request_path: path,
        response_status: session.response.status,
        measurements:
      )
    end

    def authenticate(session)
      session.get('/login')
      ensure_status!(session, 200, 'Login page')

      params = { email: profile_email, password: }
      authenticity_token = login_authenticity_token(session.response.body)
      params[:authenticity_token] = authenticity_token if authenticity_token
      session.post('/login', params:)

      return if session.response.redirect?

      raise "Login failed with HTTP #{session.response.status}"
    end

    def login_authenticity_token(body)
      Nokogiri::HTML5(body).at_css('input[name="authenticity_token"]')&.[]('value')
    end

    def request_path(household)
      application.routes.url_helpers.dashboard_path(
        household_slug: household.slug,
        dashboard_person_id: selected_person_id
      )
    end

    def capture_measurements(session, path)
      current_measurement = nil
      sql_subscriber = lambda do |*, payload|
        current_measurement[:sql_count] += 1 if current_measurement && count_sql?(payload)
      end

      ActiveSupport::Notifications.subscribed(sql_subscriber, SQL_EVENT) do
        warmup_iterations.times { perform_request(session, path) }
        measured_iterations.times.map do
          current_measurement = { sql_count: 0 }
          measure_request(session, path, current_measurement)
        ensure
          current_measurement = nil
        end
      end
    end

    def measure_request(session, path, current_measurement)
      allocations_before = allocation_counter.call
      started_at = clock.call
      perform_request(session, path)
      elapsed_ms = (clock.call - started_at) * 1000
      allocations = allocation_counter.call - allocations_before

      Measurement.new(elapsed_ms:, sql_count: current_measurement.fetch(:sql_count), allocations:)
    end

    def perform_request(session, path)
      session.get(path)
      ensure_status!(session, 200, 'Dashboard request')
    end

    def ensure_status!(session, expected_status, request_name)
      return if session.response.status == expected_status

      raise "#{request_name} returned HTTP #{session.response.status}"
    end

    def count_sql?(payload)
      !payload[:cached] && IGNORED_SQL_NAMES.exclude?(payload[:name])
    end
  end
end
