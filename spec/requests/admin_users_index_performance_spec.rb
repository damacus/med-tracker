# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin users index performance' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }
  let(:household) { households(:fixture_household) }
  let(:performance_budget_seconds) { 0.5 }

  before do
    sign_in(admin)
    create_users(100)
  end

  it 'loads within the performance budget with more than 100 users' do
    get admin_users_path

    started_at = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    get admin_users_path
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started_at

    expect(response).to have_http_status(:ok)
    expect(User.count).to be > 100
    expect(elapsed).to be < performance_budget_seconds
  end

  def create_users(count)
    count.times { |index| create_user(index) }
  end

  def create_user(index)
    account = Account.create!(
      email: "performance.user.#{index}@example.com",
      password_hash: accounts(:admin).password_hash,
      status: :verified
    )
    person = Person.create!(account: account, household: household, name: "Performance User #{index}",
                            date_of_birth: 30.years.ago.to_date)
    household.household_memberships.create!(account: account, person: person, role: :member, status: :active)
    User.create!(person: person, email_address: account.email, password_digest: admin.password_digest)
  end
end
