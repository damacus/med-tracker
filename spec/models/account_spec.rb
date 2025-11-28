# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  describe 'associations' do
    it { is_expected.to have_one(:person).dependent(:nullify) }
  end

  # rubocop:disable RSpec/NamedSubject
  describe 'validations' do
    subject { described_class.new(email: 'test@example.com') }

    it 'validates presence of email' do
      skip 'Will fix in next phase'
      expect(subject).to validate_presence_of(:email)
    end

    it 'validates uniqueness of email' do
      skip 'Will fix in next phase'
      expect(subject).to validate_uniqueness_of(:email)
    end
  end
  # rubocop:enable RSpec/NamedSubject

  describe 'enum' do
    it { is_expected.to define_enum_for(:status).with_values(unverified: 1, verified: 2, closed: 3) }
  end
end
