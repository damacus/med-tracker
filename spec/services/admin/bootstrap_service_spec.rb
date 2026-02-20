# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::BootstrapService do
  describe '.call' do
    before do
      User.administrator.destroy_all
    end

    let(:params) do
      {
        email: 'first.admin@example.com',
        password: 'SecureP@ssword123!',
        name: 'First Admin',
        date_of_birth: '1980-02-01'
      }
    end

    def create_existing_admin!(email: 'existing.admin@example.com')
      account = Account.create!(
        email: email,
        password_hash: BCrypt::Password.create('SecureP@ssword123!'),
        status: :verified
      )
      person = Person.create!(
        account: account,
        name: 'Existing Admin',
        date_of_birth: Date.new(1980, 1, 1),
        email: account.email,
        person_type: :adult
      )

      User.create!(person: person, email_address: account.email, role: :administrator, active: true)
    end

    it 'creates account, person, and administrator user when no admin exists' do
      expect do
        result = described_class.call(**params)

        expect(result).to be_success
        expect(result.user).to be_administrator
      end.to change(Account, :count).by(1)
                                    .and change(Person, :count).by(1)
                                                               .and change(User, :count).by(1)
    end

    it 'creates an active adult administrator linked to a verified account' do
      described_class.call(**params)

      created_user = User.find_by(email_address: params[:email])
      expect(created_user).to be_present
      expect(created_user).to be_active
      expect(created_user.person).to be_adult
      expect(created_user.person.account).to be_verified
    end

    it 'refuses to bootstrap when an administrator already exists' do
      create_existing_admin!

      result = nil
      expect { result = described_class.call(**params) }.not_to change(User, :count)

      expect(result).not_to be_success
      expect(result.error).to match(/already exists/i)
    end

    it 'refuses to bootstrap when account or user exists with same email' do
      Account.create!(
        email: params[:email],
        password_hash: BCrypt::Password.create('SecureP@ssword123!'),
        status: :verified
      )

      result = described_class.call(**params)

      expect(result).not_to be_success
      expect(result.error).to match(/already taken|already exists/i)
    end

    it 'rolls back account and person when user creation fails' do
      allow(User).to receive(:create!).and_raise(ActiveRecord::RecordInvalid.new(User.new))

      account_count = Account.count
      person_count = Person.count
      user_count = User.count
      result = described_class.call(**params)

      expect(result).not_to be_success
      expect(Account.count).to eq(account_count)
      expect(Person.count).to eq(person_count)
      expect(User.count).to eq(user_count)
    end
  end
end
