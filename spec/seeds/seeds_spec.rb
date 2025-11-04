# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Seeds' do
  fixtures :people, :users

  let(:email_address) { 'damacus@example.com' }
  let(:password) { 'password' }

  it 'loads the damacus admin user from fixtures' do
    user = User.find_by(email_address: email_address)
    expect(user.authenticate(password)).to be_truthy
  end

  it 'loads an admin user' do
    user = User.find_by(email_address: email_address)
    expect(user).to be_administrator
  end

  it 'has the damacus person associated with the user' do
    user = User.find_by(email_address: email_address)
    expect(user.person.email).to eq(email_address)
  end
end
