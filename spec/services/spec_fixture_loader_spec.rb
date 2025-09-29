# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SpecFixtureLoader do
  describe ".load" do
    before do
      User.destroy_all
    end

    # rubocop:disable RSpec/MultipleExpectations
    it 'loads the requested fixtures into the database' do
      expect { described_class.load(:users) }.to change(User, :count).from(0).to(6)

      user = User.find_by(email_address: 'john.doe@example.com')
      expect(user).to be_present
      expect(user.authenticate('password')).to eq(user)
    end
    # rubocop:enable RSpec/MultipleExpectations
  end
end
