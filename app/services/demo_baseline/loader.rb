# frozen_string_literal: true

module DemoBaseline
  class Loader
    class InvalidBaselineError < StandardError; end

    class << self
      delegate :load!, to: :new
    end

    def initialize(connection: ActiveRecord::Base.connection)
      @connection = connection
    end

    def load!
      with_demo_tenant_context do
        fixture_sets.each do |fixture_set|
          fixture_set.table_rows.each do |table_name, rows|
            connection.insert_fixture(rows, table_name) if rows.any?
          end
          connection.reset_pk_sequence!(fixture_set.table_name)
        end

        verify_without_tenant_context!
      end
    end

    def verify!
      with_demo_tenant_context { verify_without_tenant_context! }
    end

    private

    attr_reader :connection

    def fixture_sets
      ActiveRecord::FixtureSet.reset_cache
      FIXTURE_NAMES.map do |fixture_name|
        ActiveRecord::FixtureSet.new(nil, fixture_name, nil, FIXTURES_PATH.join(fixture_name))
      end
    end

    def with_demo_tenant_context
      connection.transaction(requires_new: true) do
        connection.select_value(
          "SELECT set_config('med_tracker.current_household_id', #{connection.quote(demo_household_id.to_s)}, true)"
        )
        yield
      end
    end

    def demo_household_id
      ActiveRecord::FixtureSet.identify(:demo_household)
    end

    def verify_without_tenant_context!
      summary = baseline_summary
      raise InvalidBaselineError, 'baseline counts do not match the committed contract' unless valid_summary?(summary)
      raise InvalidBaselineError, 'baseline contains delivery registrations or credentials' unless delivery_state_empty?

      summary
    end

    def baseline_summary
      {
        baseline: IDENTIFIER,
        accounts: Account.count,
        households: Household.count,
        people: Person.count,
        medications: Medication.count,
        schedules: Schedule.count,
        as_needed_medications: PersonMedication.as_needed.count,
        medication_takes: MedicationTake.count
      }
    end

    def valid_summary?(summary)
      summary == {
        baseline: IDENTIFIER,
        accounts: 2,
        households: 1,
        people: 3,
        medications: 2,
        schedules: 1,
        as_needed_medications: 1,
        medication_takes: 2
      }
    end

    def delivery_state_empty?
      [PushSubscription, NativeDeviceToken, ApiAppToken, ApiSession, ActiveStorage::Attachment,
       ActiveStorage::Blob].none?(&:exists?)
    end
  end
end
