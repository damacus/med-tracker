# frozen_string_literal: true

require 'rails_helper'

RSpec.describe InvitationDependent do
  let(:invitation) { create(:invitation) }
  let(:carer) { create(:person) }

  def create_minor
    person = Person.new(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    person.carer_relationships.build(carer: carer, relationship_type: 'parent')
    person.save!
    person
  end

  def create_dependent_adult
    person = Person.new(name: 'Dependent Adult', date_of_birth: 40.years.ago, person_type: :dependent_adult)
    person.carer_relationships.build(carer: carer, relationship_type: 'family_member')
    person.save!
    person
  end

  it 'is valid for a minor without capacity' do
    expect(described_class.new(invitation: invitation, dependent: create_minor)).to be_valid
  end

  it 'is valid for a dependent adult without capacity' do
    expect(described_class.new(invitation: invitation, dependent: create_dependent_adult)).to be_valid
  end

  it 'is invalid for an adult with capacity' do
    record = described_class.new(invitation: invitation, dependent: create(:person))
    expect(record).not_to be_valid
    expect(record.errors[:dependent]).to be_present
  end

  it 'is invalid for a minor that somehow has capacity' do
    minor = create_minor
    minor.update_columns(has_capacity: true) # rubocop:disable Rails/SkipsModelValidations
    expect(described_class.new(invitation: invitation, dependent: minor)).not_to be_valid
  end

  it 'enforces uniqueness of dependent within an invitation' do
    dependent = create_minor
    described_class.create!(invitation: invitation, dependent: dependent)
    expect(described_class.new(invitation: invitation, dependent: dependent)).not_to be_valid
  end
end
