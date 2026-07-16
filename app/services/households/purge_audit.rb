# frozen_string_literal: true

module Households
  class PurgeAudit
    class << self
      def attempt_started(run:, household:, actor_account:)
        first_attempt = run.attempts == 1
        record(
          run: run,
          household: household,
          actor_account: actor_account,
          event: {
            type: first_attempt ? 'household.purge.initiated' : 'household.purge.retry_started',
            outcome: first_attempt ? 'initiated' : 'retry_started'
          }
        )
      end

      def cutoff(run:, household:, actor_account:)
        record(
          run: run,
          household: household,
          actor_account: actor_account,
          event: { type: 'household.purge.cutoff', outcome: 'cutoff' }
        )
      end

      def failed(run:, household:, actor_account:, exception:)
        record(
          run: run,
          household: household,
          actor_account: actor_account,
          event: {
            type: 'household.purge.failed',
            outcome: 'failed',
            failure: {
              exception_class: exception.class.name,
              failure_code: run.failure_code
            }
          }
        )
      end

      def completed(run:, household:, actor_account:)
        record(
          run: run,
          household: household,
          actor_account: actor_account,
          event: { type: 'household.purge.completed', outcome: 'success' }
        )
      end

      private

      def record(run:, household:, actor_account:, event:)
        Audit::Event.record!(
          household: household,
          actor_account: actor_account,
          event_type: event.fetch(:type),
          metadata: metadata(run, household, event.fetch(:outcome), event.fetch(:failure, {}))
        )
      end

      def metadata(run, household, outcome, failure)
        {
          purge_run_id: run.id,
          household_id: household.id,
          attempt_number: run.attempts,
          last_completed_table: run.last_completed_table,
          outcome: outcome,
          **failure
        }.compact
      end
    end
  end
end
