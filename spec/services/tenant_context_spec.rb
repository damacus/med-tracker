# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TenantContext do
  let(:account) { Account.create!(email: 'tenant-context@example.test', status: :verified) }
  let(:household) do
    Household.create_with_owner!(
      name: 'Tenant Context Family',
      owner_account: account,
      owner_person_attributes: {
        name: 'Tenant Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end
  let(:membership) { household.household_memberships.sole }

  def current_setting(name)
    ActiveRecord::Base.connection.select_value(
      ActiveRecord::Base.sanitize_sql_array(['SELECT current_setting(?, true)', name])
    )
  end

  def current_function_value(function_name)
    ActiveRecord::Base.connection.select_value("SELECT med_tracker.#{function_name}()")
  end

  def current_context_snapshot
    [Current.account, Current.household, Current.membership, Current.request_id]
  end

  def tenant_function_snapshot
    %w[current_account_id current_household_id current_membership_id].map do |function_name|
      current_function_value(function_name)
    end
  end

  def tenant_setting_snapshot
    %w[
      med_tracker.current_account_id
      med_tracker.current_household_id
      med_tracker.current_membership_id
    ].map do |setting_name|
      current_setting(setting_name)
    end
  end

  after { Current.reset }

  it 'sets Ruby and PostgreSQL tenant context transaction-locally' do
    described_class.with(account: account, household: household, membership: membership, request_id: 'req-tenant') do
      expect(current_context_snapshot).to eq([account, household, membership, 'req-tenant'])
      expect(tenant_function_snapshot).to eq([account.id, household.id, membership.id])
    end

    expect(Current.account).to be_nil
    expect(tenant_setting_snapshot).to all(be_blank)
  end

  it 'allows account-only context before a household membership is resolved' do
    account = Account.create!(email: 'account-only-tenant-context@example.test', status: :verified)

    described_class.with(account: account, household: nil, request_id: 'req-account-only') do
      expect(current_context_snapshot).to eq([account, nil, nil, 'req-account-only'])
      expect(tenant_function_snapshot).to eq([account.id, nil, nil])
    end
  end
end
