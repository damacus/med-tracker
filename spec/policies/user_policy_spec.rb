# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe UserPolicy do
  subject(:policy) { described_class.new(current_user, user) }

  let(:user) { User.new(person: Person.new(name: 'Test User', date_of_birth: 20.years.ago)) }

  context 'when user is an administrator' do
    let(:current_user) do
      User.new(role: :administrator, person: Person.new(name: 'Admin', date_of_birth: 30.years.ago))
    end

    it 'permits all actions' do
      %i[index show create new update edit destroy activate verify].each do |action|
        expect(policy.public_send("#{action}?")).to be true
      end
    end
  end

  context 'when user is a doctor' do
    let(:current_user) do
      User.new(role: :doctor, person: Person.new(name: 'Doctor', date_of_birth: 30.years.ago))
    end

    it 'forbids administrative actions' do
      %i[index create new update edit destroy verify].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end

    context 'when viewing own profile' do
      let(:user) { current_user }

      it 'permits viewing and updating' do
        %i[show update edit].each { |action| expect(policy.public_send("#{action}?")).to be true }
      end
    end

    context 'when viewing another user' do
      it 'forbids viewing and updating' do
        %i[show update edit].each { |action| expect(policy.public_send("#{action}?")).to be false }
      end
    end
  end

  context 'when user is a nurse' do
    let(:current_user) do
      User.new(role: :nurse, person: Person.new(name: 'Nurse', date_of_birth: 30.years.ago))
    end

    it 'forbids administrative actions' do
      %i[index create new destroy verify].each { |action| expect(policy.public_send("#{action}?")).to be false }
    end

    context 'when viewing own profile' do
      let(:user) { current_user }

      it 'permits viewing and updating' do
        %i[show update edit].each { |action| expect(policy.public_send("#{action}?")).to be true }
      end
    end

    context 'when viewing another user' do
      it 'forbids viewing and updating' do
        %i[show update edit].each { |action| expect(policy.public_send("#{action}?")).to be false }
        expect(policy.update?).to be false
      end
    end
  end

  context 'when user is a carer' do
    let(:current_user) do
      User.new(role: :carer, person: Person.new(name: 'Carer', date_of_birth: 30.years.ago))
    end

    it 'forbids administrative actions' do
      expect(policy.index?).to be false
      expect(policy.create?).to be false
      expect(policy.destroy?).to be false
      expect(policy.verify?).to be false
    end

    context 'when viewing own profile' do
      let(:user) { current_user }

      it 'permits viewing and updating' do
        expect(policy.show?).to be true
        expect(policy.update?).to be true
      end
    end

    context 'when viewing another user' do
      it 'forbids viewing and updating' do
        expect(policy.show?).to be false
        expect(policy.update?).to be false
      end
    end
  end

  context 'when user is a parent' do
    let(:current_user) do
      User.new(role: :parent, person: Person.new(name: 'Parent', date_of_birth: 30.years.ago))
    end

    it 'forbids administrative actions' do
      expect(policy.index?).to be false
      expect(policy.create?).to be false
      expect(policy.destroy?).to be false
      expect(policy.verify?).to be false
    end

    context 'when viewing own profile' do
      let(:user) { current_user }

      it 'permits viewing and updating' do
        expect(policy.show?).to be true
        expect(policy.update?).to be true
      end
    end

    context 'when viewing another user' do
      it 'forbids viewing and updating' do
        expect(policy.show?).to be false
        expect(policy.update?).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { described_class::Scope.new(current_user, User.all).resolve }

    fixtures :accounts, :people, :users

    context 'when user is an administrator' do
      let(:current_user) { users(:admin) }

      it 'returns all users' do
        expect(scope).to eq(User.all)
      end
    end

    context 'when user is not an administrator' do
      let(:current_user) { users(:bob) }

      it 'returns only the current user' do
        expect(scope).to eq([current_user])
      end
    end
  end
end
