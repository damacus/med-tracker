# frozen_string_literal: true

module Households
  class LifecycleCutoffLock
    LOCK_NAMESPACE = 'med_tracker.household_purge'

    class << self
      def with(household: nil, household_id: nil)
        target_id = household_id || household&.id
        connection = ActiveRecord::Base.connection
        lock_key = "#{LOCK_NAMESPACE}:#{target_id}"
        acquired = false

        acquire(connection, lock_key)
        acquired = true
        yield
      ensure
        release(connection, lock_key) if acquired
      end

      private

      def acquire(connection, lock_key)
        connection.select_value(
          ActiveRecord::Base.sanitize_sql_array(
            [
              'WITH lock_acquired AS MATERIALIZED (' \
              'SELECT pg_advisory_lock(hashtextextended(?, 0))' \
              ') SELECT 1 FROM lock_acquired',
              lock_key
            ]
          )
        )
      end

      def release(connection, lock_key)
        connection.select_value(
          ActiveRecord::Base.sanitize_sql_array(
            ['SELECT pg_advisory_unlock(hashtextextended(?, 0))', lock_key]
          )
        )
      end
    end
  end
end
