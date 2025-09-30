# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Person, type: :model do
  subject(:person) do
    described_class.new(
      name: 'Jane Smith',
      email: 'jane.smith@example.com',
      date_of_birth: Date.new(2010, 6, 15)
    )
  end

  describe 'associations' do
    it { is_expected.to have_one(:user).inverse_of(:person).dependent(:destroy) }
    it { is_expected.to have_many(:prescriptions).dependent(:destroy) }
    it { is_expected.to have_many(:medicines).through(:prescriptions) }
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:name) }
    it { is_expected.to validate_presence_of(:date_of_birth) }
    it { is_expected.to validate_uniqueness_of(:email).case_insensitive }

    it 'allows a blank email' do
      person.email = ''
      expect(person).to be_valid
    end

    it 'enforces case-insensitive uniqueness when email is present' do
      described_class.create!(
        name: 'Existing Person',
        email: 'duplicate@example.com',
        date_of_birth: Date.new(1990, 1, 1)
      )

      person.email = 'DUPLICATE@example.com'
      expect(person).not_to be_valid
      expect(person.errors[:email]).to include('has already been taken')
    end

    it 'allows a person without a user account' do
      expect(person).to be_valid
    end
  end
end
