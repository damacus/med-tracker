# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Account-Person Associations' do
  fixtures :accounts, :people

  it 'associates Account fixtures with Person fixtures correctly' do
    # Get the damacus account fixture
    account = Account.find_by(email: 'damacus@example.com')

    # Verify account exists
    expect(account).to be_present
    expect(account.email).to eq('damacus@example.com')

    # Verify person association exists
    person = account.person
    expect(person).to be_present
    expect(person.name).to eq('Damacus User')
    expect(person.email).to eq('damacus@example.com')

    # Test the reverse association
    expect(person.account).to eq(account)
  end
end
