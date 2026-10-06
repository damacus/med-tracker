require 'rails_helper'

RSpec.describe 'Request test setup' do
  fixtures :accounts, :people, :users

  it 'establishes an authenticated session without rendering the landing page' do
    sign_in(users(:admin))

    expect(response).to have_http_status(:redirect)

    get admin_users_path

    expect(response).to have_http_status(:ok)
  end

  it 'does not rewrite people who already belong to the requested household' do
    person = people(:john)
    household = households(:fixture_household)
    assign_household(Person.where(id: person.id), household)
    statements = []
    subscriber = ->(event) { statements << event.payload.fetch(:sql) }

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') do
      assign_household(Person.where(id: person.id), household)
    end

    expect(statements.grep(/UPDATE\s+"people"/i)).to be_empty
    expect(person.reload.household).to eq(household)
  end

  it 'moves only the selected records from another household' do
    household = households(:fixture_household)
    location = Location.create!(name: 'Setup location', household: household)
    unchanged_location = Location.create!(name: 'Unchanged location', household: household)
    other_household = Household.create!(name: 'Other', slug: 'other-test-setup')

    assign_household(Location.where(id: location.id), other_household)
    expect(location.reload.household).to eq(other_household)
    expect(unchanged_location.reload.household).to eq(household)

    assign_household(Location.where(id: location.id), household)
    expect(location.reload.household).to eq(household)
  end
end
