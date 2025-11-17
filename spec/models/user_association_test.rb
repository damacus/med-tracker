# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'User Associations' do
  fixtures :accounts, :people, :users

  it 'correctly associates Account -> Person -> User for jane.doe@example.com' do
    # Get the jane_doe account
    account = Account.find_by(email: 'jane.doe@example.com')
    expect(account).to be_present
    expect(account.status).to eq('verified')

    # Verify person association
    person = account.person
    expect(person).to be_present
    expect(person.name).to eq('Jane Doe')
    expect(person.email).to eq('jane.doe@example.com')

    # Verify user association
    user = person.user
    expect(user).to be_present
    expect(user.email_address).to eq('jane.doe@example.com')
    expect(user.name).to eq('Jane Doe') # delegated method
    expect(user.role).to eq('parent')
  end
end
