# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Location do
  subject(:location) { described_class.new(name: 'Home') }

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }
    it { is_expected.to validate_uniqueness_of(:name).case_insensitive }
  end

  describe 'associations' do
    it { is_expected.to have_many(:medications).dependent(:destroy) }
    it { is_expected.to have_many(:location_memberships).dependent(:destroy) }
    it { is_expected.to have_many(:members).through(:location_memberships).source(:person) }
  end
end
