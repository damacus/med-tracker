# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  fixtures :accounts, :people

  it 'finds the damacus account fixture' do
    account = described_class.find_by(email: 'damacus@example.com')
    expect(account).to be_present
    expect(account.email).to eq('damacus@example.com')
  end

  it 'associates account with person correctly' do
    account = described_class.find_by(email: 'damacus@example.com')
    person = account.person
    expect(person).to be_present
    expect(person.name).to eq('Damacus User')
    expect(person.email).to eq('damacus@example.com')
  end

  it 'associates person with account correctly' do
    account = described_class.find_by(email: 'damacus@example.com')
    person = account.person
    expect(person.account).to eq(account)
  end
end
