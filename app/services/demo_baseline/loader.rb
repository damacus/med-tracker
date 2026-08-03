# frozen_string_literal: true

module DemoBaseline
  class Loader
    class InvalidBaselineError < StandardError; end

    class << self
      def load!
        ActiveRecord::FixtureSet.reset_cache
        ActiveRecord::FixtureSet.create_fixtures(FIXTURES_PATH, FIXTURE_NAMES)
        new.verify!
      end
    end

    def verify!
      summary = baseline_summary
      raise InvalidBaselineError, 'baseline counts do not match the committed contract' unless valid_summary?(summary)
      raise InvalidBaselineError, 'baseline contains delivery registrations or credentials' unless delivery_state_empty?

      summary
    end

    private

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
