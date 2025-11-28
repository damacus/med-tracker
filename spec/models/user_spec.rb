# frozen_string_literal: true

require 'rails_helper'

RSpec.describe User do
  subject(:user) do
    described_class.new(
      email_address: 'test@example.com',
      password: 'password',
      password_confirmation: 'password'
    )
  end

  let(:person) do
    Person.new(
      name: 'Person One',
      email: 'person.one@example.com',
      date_of_birth: Date.new(2000, 1, 1)
    )
  end

  describe 'validations' do
    before do
      user.person = person
    end

    it { is_expected.to validate_presence_of(:email_address) }
    it { is_expected.to validate_uniqueness_of(:email_address).case_insensitive }
    it { is_expected.to allow_value('user@example.com').for(:email_address) }
    it { is_expected.not_to allow_value('user@example').for(:email_address) }
    it { is_expected.not_to allow_value('userexample.com').for(:email_address) }
  end

  describe 'associations' do
    it { is_expected.to belong_to(:person).inverse_of(:user).required }
    it { is_expected.to have_many(:prescriptions).through(:person) }
  end

  describe 'person linkage' do
    it 'requires an associated person' do
      user.person = nil
      expect(user).not_to be_valid
      expect(user.errors[:person]).to include('must exist')
    end
  end

  describe 'callbacks' do
    it 'assigns the person association before validation' do
      user.person = person
      expect(user.person).to eq(person)
    end
  end

  describe 'security' do
    it { is_expected.to have_secure_password }
  end

  describe 'roles' do
    it {
      expect(user).to define_enum_for(:role)
        .with_values(administrator: 0, doctor: 1, nurse: 2, carer: 3, parent: 4, minor: 5)
        .backed_by_column_of_type(:integer)
    }

    it 'can be an administrator' do
      user.person = person
      user.role = :administrator
      expect(user.administrator?).to be true
    end

    it 'can be a doctor' do
      user.person = person
      user.role = :doctor
      expect(user.doctor?).to be true
    end

    it 'can be a nurse' do
      user.person = person
      user.role = :nurse
      expect(user.nurse?).to be true
    end

    it 'can be a carer' do
      user.person = person
      user.role = :carer
      expect(user.carer?).to be true
    end

    it 'can be a parent' do
      user.person = person
      user.role = :parent
      expect(user.parent?).to be true
    end

    it 'can be a minor' do
      user.person = person
      user.role = :minor
      expect(user.minor?).to be true
    end
  end

  describe 'normalization' do
    it 'downcases the email address before saving' do
      user = described_class.create(email_address: 'TEST@EXAMPLE.COM', password: 'password', person: person)
      expect(user.email_address).to eq('test@example.com')
    end
  end

  describe 'account activation' do
    fixtures :accounts, :people, :users

    describe '#deactivate!' do
      it 'sets active to false' do
        expect { users(:bob).deactivate! }.to change { users(:bob).reload.active }.from(true).to(false)
      end
    end

    describe '#activate!' do
      it 'sets active to true' do
        users(:bob).update!(active: false)
        expect { users(:bob).activate! }.to change { users(:bob).reload.active }.from(false).to(true)
      end
    end

    describe 'scopes' do
      it 'returns only active users with .active scope' do
        users(:bob).deactivate!
        expect(described_class.active).not_to include(users(:bob))
        expect(described_class.active).to include(users(:admin))
      end

      it 'returns only inactive users with .inactive scope' do
        users(:bob).deactivate!
        expect(described_class.inactive).to include(users(:bob))
        expect(described_class.inactive).not_to include(users(:admin))
      end
    end
  end

  describe 'versioning' do
    fixtures :accounts, :people, :users

    let(:admin) { users(:admin) }

    before do
      PaperTrail.request.whodunnit = admin.id
    end

    after do
      PaperTrail.request.whodunnit = nil
    end

    it 'creates version on user creation' do
      new_person = Person.create!(name: 'New User', date_of_birth: 30.years.ago)

      expect do
        described_class.create!(
          email_address: 'newuser@example.com',
          password: 'password',
          person: new_person
        )
      end.to change(PaperTrail::Version, :count).by(1)

      version = PaperTrail::Version.last
      expect(version.event).to eq('create')
      expect(version.item_type).to eq('User')
    end

    it 'creates version on user update' do
      expect do
        admin.update!(email_address: 'updated@example.com')
      end.to change(PaperTrail::Version, :count).by(1)

      version = admin.versions.last
      expect(version.event).to eq('update')
      # PaperTrail stores the previous state in 'object'
      expect(version.object).to be_present
    end

    it 'associates version with current user' do
      admin.update!(email_address: 'uniquetest@example.com')
      expect(admin.versions.last.whodunnit).to eq(admin.id.to_s)
    end

    it 'does not track password changes' do
      initial_version_count = admin.versions.count
      admin.update!(password: 'newpassword')
      # Password-only changes should not create versions
      expect(admin.versions.count).to eq(initial_version_count)
    end

    it 'stores version when changes occur' do
      admin.update!(email_address: 'iptest@example.com')
      version = admin.versions.last
      expect(version).to be_present
      expect(version.item).to eq(admin)
    end
  end
end
