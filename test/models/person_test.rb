# frozen_string_literal: true

require 'test_helper'

class PersonTest < ActiveSupport::TestCase
  # -- associations --

  test 'has_one user' do
    person = people(:john)
    assert_respond_to person, :user
    assert_instance_of User, person.user
  end

  test 'has_many prescriptions' do
    person = people(:john)
    assert_respond_to person, :prescriptions
    assert_kind_of ActiveRecord::Associations::CollectionProxy, person.prescriptions
  end

  test 'has_many medicines through prescriptions' do
    person = people(:john)
    assert_respond_to person, :medicines
  end

  test 'has_many carer_relationships' do
    person = people(:john)
    assert_respond_to person, :carer_relationships
  end

  test 'has_many carers through active_carer_relationships' do
    person = people(:john)
    assert_respond_to person, :carers
  end

  test 'has_many patient_relationships' do
    person = people(:john)
    assert_respond_to person, :patient_relationships
  end

  test 'has_many patients through active_patient_relationships' do
    person = people(:nurse_smith)
    assert_respond_to person, :patients
  end

  # -- validations --

  test 'requires name' do
    person = Person.new(date_of_birth: 25.years.ago)
    assert_not person.valid?
    assert_includes person.errors[:name], "can't be blank"
  end

  test 'requires date_of_birth' do
    person = Person.new(name: 'Test')
    assert_not person.valid?
    assert_includes person.errors[:date_of_birth], "can't be blank"
  end

  test 'validates email uniqueness case-insensitively' do
    Person.create!(name: 'Existing', email: 'duplicate@example.com', date_of_birth: 30.years.ago)
    person = Person.new(name: 'New', email: 'DUPLICATE@example.com', date_of_birth: 25.years.ago)
    assert_not person.valid?
    assert_includes person.errors[:email], 'has already been taken'
  end

  test 'allows blank email' do
    person = Person.new(name: 'No Email', email: '', date_of_birth: 25.years.ago)
    assert person.valid?
  end

  test 'allows person without user account' do
    person = Person.new(name: 'Solo', date_of_birth: 25.years.ago)
    assert person.valid?
  end

  # -- person types --

  test 'defaults to adult type' do
    person = Person.create!(name: 'Default', date_of_birth: 20.years.ago)
    assert person.adult?
  end

  test 'can be a minor' do
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    assert_equal 'minor', person.person_type
  end

  test 'can be a dependent adult' do
    person = Person.create!(name: 'Dependent', date_of_birth: 75.years.ago, person_type: :dependent_adult)
    assert_equal 'dependent_adult', person.person_type
  end

  # -- #age --

  test 'age calculates correctly' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    assert_equal 25, person.age
  end

  test 'age handles birthdays correctly' do
    today = Date.new(2024, 6, 15)
    person = Person.new(name: 'Test', date_of_birth: Date.new(2000, 6, 16))
    assert_equal 23, person.age(today)
  end

  test 'age returns nil when date_of_birth is nil' do
    person = Person.new(name: 'Test')
    assert_nil person.age
  end

  # -- #adult? --

  test 'adult? returns true for person 18+ with adult person_type' do
    person = Person.new(name: 'Adult', date_of_birth: 25.years.ago, person_type: :adult)
    assert person.adult?
  end

  test 'adult? returns true for adult person_type regardless of age' do
    person = Person.new(name: 'Young Adult', date_of_birth: 10.years.ago, person_type: :adult)
    assert person.adult?
  end

  test 'adult? returns true for 18+ even with minor person_type' do
    person = Person.new(name: 'Grown Minor', date_of_birth: 18.years.ago, person_type: :minor)
    assert person.adult?
  end

  test 'adult? returns false for under-18 without adult person_type' do
    person = Person.new(name: 'Kid', date_of_birth: 10.years.ago, person_type: :minor)
    assert_not person.adult?
  end

  # -- #minor? --

  test 'minor? returns true for under-18 with minor person_type' do
    person = Person.new(name: 'Kid', date_of_birth: 10.years.ago, person_type: :minor)
    assert person.minor?
  end

  test 'minor? returns false for 18+ even with minor person_type' do
    person = Person.new(name: 'Grown', date_of_birth: 25.years.ago, person_type: :minor)
    assert_not person.minor?
  end

  test 'minor? returns false for under-18 without minor person_type' do
    person = Person.new(name: 'Young Adult Type', date_of_birth: 10.years.ago, person_type: :adult)
    assert_not person.minor?
  end

  test 'minor? returns false for exactly 18' do
    person = Person.new(name: 'Just 18', date_of_birth: 18.years.ago, person_type: :minor)
    assert_not person.minor?
  end

  # -- #dependent_adult? --

  test 'dependent_adult? returns true for 18+ with dependent_adult type' do
    person = Person.new(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    assert person.dependent_adult?
  end

  test 'dependent_adult? returns false for under-18 with dependent_adult type' do
    person = Person.new(name: 'Young Dependent', date_of_birth: 10.years.ago, person_type: :dependent_adult)
    assert_not person.dependent_adult?
  end

  test 'dependent_adult? returns false for 18+ without dependent_adult type' do
    person = Person.new(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    assert_not person.dependent_adult?
  end

  # -- capacity --

  test 'has capacity by default' do
    person = Person.create!(name: 'Capable', date_of_birth: 20.years.ago)
    assert person.has_capacity
  end

  test 'can lack capacity when carer is assigned' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    person.carer_relationships.build(carer: carer, relationship_type: 'parent')
    person.save!

    assert_not person.has_capacity
    assert person.valid?
  end

  # -- carer requirement validation --

  test 'requires carer when has_capacity is false and no carers' do
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  test 'valid when has_capacity is false and carer assigned' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    person.carer_relationships.build(carer: carer, relationship_type: 'parent')
    assert person.valid?
  end

  test 'valid when has_capacity is true without carers' do
    person = Person.new(name: 'Capable', date_of_birth: 30.years.ago, has_capacity: true)
    assert person.valid?
  end

  test 'prevents removing capacity when no carers assigned' do
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.has_capacity = false
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  test 'allows removing capacity when carer already assigned' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support')
    person.has_capacity = false
    assert person.valid?
  end

  test 'requires active carer when only inactive carers exist' do
    carer = Person.create!(name: 'Inactive Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support', active: false)
    person.has_capacity = false
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  # -- carer relationships --

  test 'can have carers assigned' do
    carer = Person.create!(name: 'Parent Carer', date_of_birth: 35.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer, relationship_type: 'parent')
    patient.save!

    assert_includes patient.carers, carer
    assert_includes carer.patients, patient
  end

  test 'can have multiple carers' do
    carer1 = Person.create!(name: 'Carer 1', date_of_birth: 35.years.ago, person_type: :adult)
    carer2 = Person.create!(name: 'Carer 2', date_of_birth: 40.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer1, relationship_type: 'parent')
    patient.save!
    patient.carer_relationships.create!(carer: carer2, relationship_type: 'guardian')
    assert_equal 2, patient.carers.count
  end

  test 'can specify relationship type' do
    carer = Person.create!(name: 'Carer', date_of_birth: 35.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer, relationship_type: 'parent')
    patient.save!
    assert_equal 'parent', patient.carer_relationships.first.relationship_type
  end

  test 'active_carer_relationships only returns active relationships' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_equal 1, person.active_carer_relationships.count
    assert_equal active_carer, person.active_carer_relationships.first.carer
  end

  test 'carer_relationships returns all relationships' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    carer1 = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    carer2 = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: carer1, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: carer2, patient: person, relationship_type: 'professional_carer', active: false)

    assert_equal 2, person.carer_relationships.count
  end

  test 'carers only returns carers with active relationships' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_includes person.carers, active_carer
    assert_not_includes person.carers, inactive_carer
  end

  test 'active_patient_relationships only returns active' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_equal 1, active_carer.active_patient_relationships.count
    assert_equal 0, inactive_carer.active_patient_relationships.count
  end

  test 'patients only returns patients with active relationships' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_includes active_carer.patients, person
    assert_not_includes inactive_carer.patients, person
  end

  # -- #needs_carer? --

  test 'needs_carer? returns false for adult type without carers' do
    person = Person.create!(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    assert_not person.needs_carer?
  end

  test 'needs_carer? returns false for adult type with carers' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support')
    assert_not person.needs_carer?
  end

  test 'needs_carer? returns true for minor without carers' do
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    assert person.needs_carer?
  end

  test 'needs_carer? returns false for minor with carers' do
    parent = Person.create!(name: 'Parent', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    person.carer_relationships.create!(carer: parent, relationship_type: 'parent')
    assert_not person.needs_carer?
  end

  test 'needs_carer? returns true for dependent_adult without carers' do
    person = Person.create!(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    assert person.needs_carer?
  end

  test 'needs_carer? returns false for dependent_adult with carers' do
    carer = Person.create!(name: 'Carer', date_of_birth: 45.years.ago, person_type: :adult)
    person = Person.create!(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    person.carer_relationships.create!(carer: carer, relationship_type: 'guardian')
    assert_not person.needs_carer?
  end

  # -- versioning (PaperTrail) --

  test 'creates version on person creation' do
    PaperTrail.request.whodunnit = users(:admin).id
    assert_difference('PaperTrail::Version.count', 1) do
      Person.create!(name: 'New Person', date_of_birth: 25.years.ago)
    end
    version = PaperTrail::Version.last
    assert_equal 'create', version.event
    assert_equal 'Person', version.item_type
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'creates version on person update' do
    PaperTrail.request.whodunnit = users(:admin).id
    john = people(:john)
    assert_difference('PaperTrail::Version.count', 1) do
      john.update!(name: 'John Updated')
    end
    version = john.versions.last
    assert_equal 'update', version.event
    assert version.object.present?
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'tracks sensitive field changes' do
    PaperTrail.request.whodunnit = users(:admin).id
    john = people(:john)
    original_dob = john.date_of_birth
    john.update!(date_of_birth: 27.years.ago.to_date)
    reified = john.versions.last.reify
    assert_equal original_dob, reified.date_of_birth
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'associates version with current user' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    john = people(:john)
    john.update!(name: 'John Modified')
    assert_equal admin.id.to_s, john.versions.last.whodunnit
  ensure
    PaperTrail.request.whodunnit = nil
  end
end
