# frozen_string_literal: true

class ApiLoginFailureRecorder
  MAX_INVALID_LOGINS = 5
  LOCKOUT_DEADLINE_INTERVAL = 30.minutes

  class << self
    def record_failure(account)
      new(account).record_failure
    end

    def clear_failures(account)
      AccountLoginFailure.where(account_id: account.id).delete_all if account
    end
  end

  def initialize(account)
    @account = account
  end

  def record_failure
    return if account.blank? || ApiAuthState.locked_out?(account)

    account.with_lock do
      failure = AccountLoginFailure.find_or_initialize_by(account: account)
      failure.number = failure.new_record? ? 1 : failure.number.to_i + 1
      failure.save!
      lock_account! if failure.number >= MAX_INVALID_LOGINS
    end
  end

  private

  attr_reader :account

  def lock_account!
    lockout = AccountLockout.find_or_initialize_by(account: account)
    lockout.key = SecureRandom.urlsafe_base64(32)
    lockout.deadline = LOCKOUT_DEADLINE_INTERVAL.from_now
    lockout.save!
  end
end
