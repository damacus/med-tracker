# frozen_string_literal: true

require 'rails_helper'

RSpec.describe User do
  fixtures :accounts, :people, :users

  it 'finds the jane_doe account and verifies status' do
    account = Account.find_by(email: 'jane.doe@example.com')
    expect(account).to be_present
    expect(account.status).to eq('verified')
  end

  it 'associates account with person correctly' do
    account = Account.find_by(email: 'jane.doe@example.com')
    person = account.person
    expect(person).to be_present
    expect(person.name).to eq('Jane Doe')
    expect(person.email).to eq('jane.doe@example.com')
  end

  it 'associates person with user correctly' do
    account = Account.find_by(email: 'jane.doe@example.com')
    person = account.person
    user = person.user
    expect(user).to be_present
    expect(user.email_address).to eq('jane.doe@example.com')
    expect(user.name).to eq('Jane Doe') # delegated method
    expect(user.role).to eq('parent')
  end
end
