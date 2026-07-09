# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiLoginFailureRecorder do
  fixtures :accounts

  let(:account) { accounts(:damacus) }

  before do
    AccountLoginFailure.where(account_id: account.id).delete_all
    AccountLockout.where(account_id: account.id).delete_all
  end

  it 'increments the login failure counter for a verified account' do
    2.times { described_class.record_failure(account) }

    expect(AccountLoginFailure.find_by!(account_id: account.id).number).to eq(2)
    expect(ApiAuthState.locked_out?(account)).to be(false)
  end

  it 'creates an active lockout on the fifth failure' do
    5.times { described_class.record_failure(account) }

    expect(ApiAuthState.locked_out?(account)).to be(true)
    expect(AccountLockout.find_by!(account_id: account.id).deadline).to be > Time.current
  end

  it 'does not record failures for blank accounts' do
    expect { described_class.record_failure(nil) }
      .not_to change(AccountLoginFailure, :count)
  end

  it 'clears existing failures after a successful login' do
    described_class.record_failure(account)

    expect { described_class.clear_failures(account) }
      .to change { AccountLoginFailure.where(account_id: account.id).count }.from(1).to(0)
  end
end
