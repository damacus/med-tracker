# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe InvitationPolicy do
  subject(:policy) { described_class.new(current_user, invitation) }

  let(:invitation) { Invitation.new(email: 'test@example.com', role: :parent) }

  context 'when user is an administrator' do
    let(:current_user) do
      User.new(role: :administrator, person: Person.new(name: 'Admin', date_of_birth: 30.years.ago))
    end

    it 'permits index' do
      expect(policy.index?).to be true
    end

    it 'permits create' do
      expect(policy.create?).to be true
    end
  end

  context 'when user is a doctor' do
    let(:current_user) do
      User.new(role: :doctor, person: Person.new(name: 'Doctor', date_of_birth: 30.years.ago))
    end

    it 'forbids index' do
      expect(policy.index?).to be false
    end

    it 'forbids create' do
      expect(policy.create?).to be false
    end
  end

  context 'when user is a nurse' do
    let(:current_user) do
      User.new(role: :nurse, person: Person.new(name: 'Nurse', date_of_birth: 30.years.ago))
    end

    it 'forbids index' do
      expect(policy.index?).to be false
    end

    it 'forbids create' do
      expect(policy.create?).to be false
    end
  end

  context 'when user is a carer' do
    let(:current_user) do
      User.new(role: :carer, person: Person.new(name: 'Carer', date_of_birth: 30.years.ago))
    end

    it 'forbids index' do
      expect(policy.index?).to be false
    end

    it 'forbids create' do
      expect(policy.create?).to be false
    end
  end

  context 'when user is a parent' do
    let(:current_user) do
      User.new(role: :parent, person: Person.new(name: 'Parent', date_of_birth: 30.years.ago))
    end

    it 'forbids index' do
      expect(policy.index?).to be false
    end

    it 'forbids create' do
      expect(policy.create?).to be false
    end
  end

  context 'when user is a minor' do
    let(:current_user) do
      User.new(role: :minor, person: Person.new(name: 'Minor', date_of_birth: 10.years.ago))
    end

    it 'forbids index' do
      expect(policy.index?).to be false
    end

    it 'forbids create' do
      expect(policy.create?).to be false
    end
  end
end
