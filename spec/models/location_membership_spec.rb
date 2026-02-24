# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationMembership do
  describe 'validations' do
    subject(:membership) { build(:location_membership) }

    it { is_expected.to validate_uniqueness_of(:person_id).scoped_to(:location_id) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:location) }
    it { is_expected.to belong_to(:person) }
  end
end
