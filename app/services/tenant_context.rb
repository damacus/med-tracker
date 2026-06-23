# frozen_string_literal: true

class TenantContext
  SETTING_NAMES = {
    account: 'med_tracker.current_account_id',
    household: 'med_tracker.current_household_id',
    membership: 'med_tracker.current_membership_id'
  }.freeze

  class << self
    def with(account:, household:, membership: nil, request_id: nil)
      previous = current_attributes

      ActiveRecord::Base.transaction(requires_new: true) do
        assign_current(account:, household:, membership:, request_id:)
        set_database_context(account:, household:, membership:)
        yield
      ensure
        clear_database_context
        restore_current(previous)
      end
    end

    def set_household!(household)
      Current.household = household
      set_local(SETTING_NAMES[:household], household&.id)
    end

    def set_membership!(membership)
      Current.membership = membership
      set_local(SETTING_NAMES[:membership], membership&.id)
    end

    private

    def assign_current(account:, household:, membership:, request_id:)
      Current.account = account
      Current.household = household
      Current.membership = membership
      Current.request_id = request_id
    end

    def current_attributes
      {
        account: Current.account,
        household: Current.household,
        membership: Current.membership,
        request_id: Current.request_id
      }
    end

    def restore_current(attributes)
      Current.account = attributes[:account]
      Current.household = attributes[:household]
      Current.membership = attributes[:membership]
      Current.request_id = attributes[:request_id]
    end

    def set_database_context(account:, household:, membership:)
      set_local(SETTING_NAMES[:account], account&.id)
      set_local(SETTING_NAMES[:household], household&.id)
      set_local(SETTING_NAMES[:membership], membership&.id)
    end

    def clear_database_context
      SETTING_NAMES.each_value { |setting_name| set_local(setting_name, nil) }
    end

    def set_local(setting_name, value)
      ActiveRecord::Base.connection.execute(
        ActiveRecord::Base.sanitize_sql_array(
          ['SELECT set_config(?, ?, true)', setting_name, value.to_s]
        )
      )
    end
  end
end
