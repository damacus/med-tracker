# frozen_string_literal: true

module SupportAccessSessions
  class Creator
    class << self
      def call(support_session:, authorize:)
        ActiveRecord::Base.transaction do
          lock_household!(support_session)
          authorize.call(support_session)
          support_session.save!
          yield support_session if block_given?
          support_session
        end
      end

      private

      def lock_household!(support_session)
        household = support_session.household
        household.lock!
        support_session.household = household
      end
    end
  end
end
