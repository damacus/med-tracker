# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Person do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:person) { people(:damacus) }

  describe 'avatar attachment' do
    it 'accepts supported image uploads' do
      person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

      expect(person).to be_valid
    end

    it 'rejects non-image uploads' do
      person.avatar.attach(io: StringIO.new('not an image'), filename: 'avatar.txt', content_type: 'text/plain')

      expect(person).not_to be_valid
      expect(person.errors[:avatar]).to include('must be a PNG, JPEG, or WebP image')
    end

    it 'rejects avatars larger than five megabytes' do
      person.avatar.attach(
        io: StringIO.new('x' * 6.megabytes),
        filename: 'avatar.png',
        content_type: 'image/png'
      )

      expect(person).not_to be_valid
      expect(person.errors[:avatar]).to include('must be smaller than 5 MB')
    end

    it 'stores avatar attachments with the person household' do
      household = Household.create!(name: 'Avatar Household', slug: 'avatar-household')
      tenant_person = household.people.create!(
        name: 'Avatar Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      )

      tenant_person.avatar.attach(io: StringIO.new('avatar'), filename: 'avatar.png', content_type: 'image/png')

      expect(tenant_person.avatar.attachment.household_id).to eq(household.id)
    end
  end
end
