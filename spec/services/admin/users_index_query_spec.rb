# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::UsersIndexQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:scope) { User.all }
  let!(:soft_deleted_user) do
    account = Account.create!(
      email: 'soft.deleted@example.com',
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :closed
    )
    person = Person.create!(
      name: 'Soft Deleted User',
      date_of_birth: '1990-01-01',
      account: account
    )
    person.update!(account: nil)
    User.create!(
      person: person,
      email_address: 'soft.deleted@example.com',
      role: :parent,
      active: true
    )
  end

  describe '#call' do
    it 'searches by person name or email' do
      result = described_class.new(scope: scope, filters: { search: 'Jane' }).call

      expect(result).to include(users(:jane))
      expect(result).not_to include(users(:bob))
    end

    it 'filters by role' do
      result = described_class.new(scope: scope, filters: { role: 'doctor' }).call

      expect(result).to contain_exactly(users(:doctor))
    end

    it 'filters soft deleted users' do
      result = described_class.new(scope: scope, filters: { status: 'soft_deleted' }).call

      expect(result).to contain_exactly(soft_deleted_user)
    end

    it 'sorts by associated person name' do
      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { sort: 'name', direction: 'asc' }
      ).call

      expect(result.map(&:name)).to eq(['Bob Smith', 'Jane Doe'])
    end

    it 'sorts by email when requested' do
      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { sort: 'email', direction: 'desc' }
      ).call

      expect(result.map(&:email_address)).to eq(['jane.doe@example.com', 'bob.smith@example.com'])
    end
  end
end
