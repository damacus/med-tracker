# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Location do
  subject(:location) { described_class.new(name: 'Home') }

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }

    it 'rejects duplicate names within the same household' do
      household = Household.create!(name: 'Shared household', slug: 'shared-household')
      create(:location, name: 'Home', household: household)

      location = build(:location, name: 'home', household: household)

      expect(location).not_to be_valid
      expect(location.errors[:name]).to include('has already been taken')
    end

    it 'allows the same name in different households' do
      first_household = Household.create!(name: 'First household', slug: 'first-household')
      second_household = Household.create!(name: 'Second household', slug: 'second-household')
      create(:location, name: 'Home', household: first_household)

      location = build(:location, name: 'Home', household: second_household)

      expect(location).to be_valid
    end
  end

  describe 'associations' do
    it { is_expected.to belong_to(:household).optional }
    it { is_expected.to have_many(:medications).dependent(:destroy) }
    it { is_expected.to have_many(:location_memberships).dependent(:destroy) }
    it { is_expected.to have_many(:members).through(:location_memberships).source(:person) }
  end

  describe 'versioning' do
    it 'creates a version when a location changes' do
      location = create(:location)

      expect do
        location.update!(name: 'Updated storage location')
      end.to change { PaperTrail::Version.where(item_type: 'Location', item_id: location.id).count }.by(1)

      expect(PaperTrail::Version.where(item_type: 'Location', item_id: location.id).last.event).to eq('update')
    end
  end
end
