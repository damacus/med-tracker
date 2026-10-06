# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SpecFixtureLoader do
  self.use_transactional_tests = false

  describe '.load' do
    before do
      PersonAccessGrant.delete_all
      HouseholdMembership.delete_all
    end

    # This test needs to run outside of transactional fixtures because
    # SpecFixtureLoader is designed for seeding, not for use within tests.
    # We test it by checking that it can load fixtures successfully.
    it 'loads the requested fixtures into the database', :aggregate_failures do
      # Load fixtures including all dependencies
      described_class.load(:accounts, :people, :users)

      # Verify fixtures were loaded
      user = User.find_by(email_address: 'john.doe@example.com')
      expect(user).to be_present
      expect(user.person).to be_present
      expect(user.authenticate('password')).to eq(user)
    end

    it 'loads account fixtures when dependent OTP keys already exist' do
      ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('spec/fixtures'), %w[accounts account_otp_keys])

      expect { described_class.load(:accounts, :people, :users) }.not_to raise_error
    end
  end
end
