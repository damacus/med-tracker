# frozen_string_literal: true

require 'rails_helper'

RSpec.describe User do
  it 'defaults to parent role for new users' do
    person = Person.create!(name: 'New User', date_of_birth: 20.years.ago)
    user = described_class.create!(
      email_address: 'newuser@example.com',
      password: 'password',
      person: person
    )

    expect(user.role).to eq('parent')
    expect(user.parent?).to be true
    expect(user.administrator?).to be false
  end

  it 'allows explicitly setting administrator role' do
    person = Person.create!(name: 'Admin User', date_of_birth: 30.years.ago)
    user = described_class.create!(
      email_address: 'explicitadmin@example.com',
      password: 'password',
      person: person,
      role: :administrator
    )

    expect(user.administrator?).to be true
  end
end
