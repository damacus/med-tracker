# frozen_string_literal: true

require 'test_helper'

class PersonTest < ActiveSupport::TestCase
  # -- associations --

  test 'a person can have a user account' do
    person = people(:john)
    assert_respond_to person, :user
    assert_instance_of User, person.user
  end

  test 'a person can have multiple prescriptions' do
    person = people(:john)
    assert_respond_to person, :prescriptions
    assert_kind_of ActiveRecord::Associations::CollectionProxy, person.prescriptions
  end

  test 'a person can access their medicines via prescriptions' do
    person = people(:john)
    assert_respond_to person, :medicines
  end

  test 'a person can have carer relationships' do
    person = people(:john)
    assert_respond_to person, :carer_relationships
  end

  test 'a person can have carers assigned to them' do
    person = people(:john)
    assert_respond_to person, :carers
  end

  test 'a person can be a carer for others' do
    person = people(:john)
    assert_respond_to person, :patient_relationships
  end

  test 'a carer can access their patients' do
    person = people(:nurse_smith)
    assert_respond_to person, :patients
  end

  # -- validations --

  test 'a person must have a name' do
    person = Person.new(date_of_birth: 25.years.ago)
    assert_not person.valid?
    assert_includes person.errors[:name], "can't be blank"
  end

  test 'a person must have a date of birth' do
    person = Person.new(name: 'Test')
    assert_not person.valid?
    assert_includes person.errors[:date_of_birth], "can't be blank"
  end

  test 'two people cannot share the same email address regardless of case' do
    Person.create!(name: 'Existing', email: 'duplicate@example.com', date_of_birth: 30.years.ago)
    person = Person.new(name: 'New', email: 'DUPLICATE@example.com', date_of_birth: 25.years.ago)
    assert_not person.valid?
    assert_includes person.errors[:email], 'has already been taken'
  end

  test 'a person does not need an email address' do
    person = Person.new(name: 'No Email', email: '', date_of_birth: 25.years.ago)
    assert person.valid?
  end

  test 'a person does not need a user account' do
    person = Person.new(name: 'Solo', date_of_birth: 25.years.ago)
    assert person.valid?
  end

  # -- person types --

  test 'new people default to adult type' do
    person = Person.create!(name: 'Default', date_of_birth: 20.years.ago)
    assert person.adult?
  end

  test 'a person can be recorded as a minor' do
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    assert_equal 'minor', person.person_type
  end

  test 'a person can be recorded as a dependent adult' do
    person = Person.create!(name: 'Dependent', date_of_birth: 75.years.ago, person_type: :dependent_adult)
    assert_equal 'dependent_adult', person.person_type
  end

  # -- #age --

  test 'age is calculated from date of birth' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    assert_equal 25, person.age
  end

  test 'age does not increment until the birthday has passed' do
    today = Date.new(2024, 6, 15)
    person = Person.new(name: 'Test', date_of_birth: Date.new(2000, 6, 16))
    assert_equal 23, person.age(today)
  end

  test 'age is nil when date of birth is missing' do
    person = Person.new(name: 'Test')
    assert_nil person.age
  end

  # -- #adult? --

  test 'a person over 18 with adult type is considered an adult' do
    person = Person.new(name: 'Adult', date_of_birth: 25.years.ago, person_type: :adult)
    assert person.adult?
  end

  test 'a person with adult type is considered an adult regardless of age' do
    person = Person.new(name: 'Young Adult', date_of_birth: 10.years.ago, person_type: :adult)
    assert person.adult?
  end

  test 'a person over 18 is considered an adult even if typed as minor' do
    person = Person.new(name: 'Grown Minor', date_of_birth: 18.years.ago, person_type: :minor)
    assert person.adult?
  end

  test 'a person under 18 without adult type is not considered an adult' do
    person = Person.new(name: 'Kid', date_of_birth: 10.years.ago, person_type: :minor)
    assert_not person.adult?
  end

  # -- #minor? --

  test 'a child under 18 typed as minor is considered a minor' do
    person = Person.new(name: 'Kid', date_of_birth: 10.years.ago, person_type: :minor)
    assert person.minor?
  end

  test 'a person over 18 is never considered a minor' do
    person = Person.new(name: 'Grown', date_of_birth: 25.years.ago, person_type: :minor)
    assert_not person.minor?
  end

  test 'a child under 18 typed as adult is not considered a minor' do
    person = Person.new(name: 'Young Adult Type', date_of_birth: 10.years.ago, person_type: :adult)
    assert_not person.minor?
  end

  test 'a person who just turned 18 is no longer a minor' do
    person = Person.new(name: 'Just 18', date_of_birth: 18.years.ago, person_type: :minor)
    assert_not person.minor?
  end

  # -- #dependent_adult? --

  test 'a person over 18 typed as dependent adult is considered dependent' do
    person = Person.new(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    assert person.dependent_adult?
  end

  test 'a person under 18 is not considered a dependent adult' do
    person = Person.new(name: 'Young Dependent', date_of_birth: 10.years.ago, person_type: :dependent_adult)
    assert_not person.dependent_adult?
  end

  test 'a regular adult is not considered a dependent adult' do
    person = Person.new(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    assert_not person.dependent_adult?
  end

  # -- capacity --

  test 'people have capacity by default' do
    person = Person.create!(name: 'Capable', date_of_birth: 20.years.ago)
    assert person.has_capacity
  end

  test 'a person can lack capacity when they have a carer' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    person.carer_relationships.build(carer: carer, relationship_type: 'parent')
    person.save!

    assert_not person.has_capacity
    assert person.valid?
  end

  # -- carer requirement validation --

  test 'a person without capacity must have at least one carer' do
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  test 'a person without capacity is valid when a carer is assigned' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.new(name: 'No Capacity', date_of_birth: 5.years.ago, has_capacity: false)
    person.carer_relationships.build(carer: carer, relationship_type: 'parent')
    assert person.valid?
  end

  test 'a person with capacity does not need a carer' do
    person = Person.new(name: 'Capable', date_of_birth: 30.years.ago, has_capacity: true)
    assert person.valid?
  end

  test 'capacity cannot be removed unless a carer is assigned first' do
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.has_capacity = false
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  test 'capacity can be removed when a carer is already assigned' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support')
    person.has_capacity = false
    assert person.valid?
  end

  test 'inactive carers do not count for capacity requirements' do
    carer = Person.create!(name: 'Inactive Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Person', date_of_birth: 30.years.ago, has_capacity: true)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support', active: false)
    person.has_capacity = false
    assert_not person.valid?
    assert_includes person.errors[:base], 'A person without capacity must have at least one carer assigned'
  end

  # -- carer relationships --

  test 'assigning a carer links both patient and carer' do
    carer = Person.create!(name: 'Parent Carer', date_of_birth: 35.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer, relationship_type: 'parent')
    patient.save!

    assert_includes patient.carers, carer
    assert_includes carer.patients, patient
  end

  test 'a patient can have more than one carer' do
    carer1 = Person.create!(name: 'Carer 1', date_of_birth: 35.years.ago, person_type: :adult)
    carer2 = Person.create!(name: 'Carer 2', date_of_birth: 40.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer1, relationship_type: 'parent')
    patient.save!
    patient.carer_relationships.create!(carer: carer2, relationship_type: 'guardian')
    assert_equal 2, patient.carers.count
  end

  test 'carer relationships record the type of relationship' do
    carer = Person.create!(name: 'Carer', date_of_birth: 35.years.ago, person_type: :adult)
    patient = Person.new(name: 'Child', date_of_birth: 5.years.ago, person_type: :minor, has_capacity: false)
    patient.carer_relationships.build(carer: carer, relationship_type: 'parent')
    patient.save!
    assert_equal 'parent', patient.carer_relationships.first.relationship_type
  end

  test 'only active carer relationships are counted as current' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_equal 1, person.active_carer_relationships.count
    assert_equal active_carer, person.active_carer_relationships.first.carer
  end

  test 'all carer relationships are retained including inactive ones' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    carer1 = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    carer2 = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: carer1, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: carer2, patient: person, relationship_type: 'professional_carer', active: false)

    assert_equal 2, person.carer_relationships.count
  end

  test 'a person only sees their currently active carers' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_includes person.carers, active_carer
    assert_not_includes person.carers, inactive_carer
  end

  test 'a carer only sees patients from active relationships' do
    person = Person.create!(name: 'Patient', date_of_birth: 30.years.ago, person_type: :adult, has_capacity: true)
    active_carer = Person.create!(name: 'Active', date_of_birth: 35.years.ago, person_type: :adult)
    inactive_carer = Person.create!(name: 'Inactive', date_of_birth: 40.years.ago, person_type: :adult)
    CarerRelationship.create!(carer: active_carer, patient: person, relationship_type: 'family_member', active: true)
    CarerRelationship.create!(carer: inactive_carer, patient: person, relationship_type: 'professional_carer',
                              active: false)

    assert_equal 1, active_carer.active_patient_relationships.count
    assert_equal 0, inactive_carer.active_patient_relationships.count
  end

  test 'inactive carer relationships are excluded from the patient list' do
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

  test 'an adult does not need a carer' do
    person = Person.create!(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    assert_not person.needs_carer?
  end

  test 'an adult with carers still does not need one' do
    carer = Person.create!(name: 'Carer', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Adult', date_of_birth: 30.years.ago, person_type: :adult)
    person.carer_relationships.create!(carer: carer, relationship_type: 'support')
    assert_not person.needs_carer?
  end

  test 'a minor without carers needs one assigned' do
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    assert person.needs_carer?
  end

  test 'a minor with a carer does not need another' do
    parent = Person.create!(name: 'Parent', date_of_birth: 40.years.ago, person_type: :adult)
    person = Person.create!(name: 'Minor', date_of_birth: 10.years.ago, person_type: :minor)
    person.carer_relationships.create!(carer: parent, relationship_type: 'parent')
    assert_not person.needs_carer?
  end

  test 'a dependent adult without carers needs one assigned' do
    person = Person.create!(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    assert person.needs_carer?
  end

  test 'a dependent adult with a carer does not need another' do
    carer = Person.create!(name: 'Carer', date_of_birth: 45.years.ago, person_type: :adult)
    person = Person.create!(name: 'Dependent', date_of_birth: 70.years.ago, person_type: :dependent_adult)
    person.carer_relationships.create!(carer: carer, relationship_type: 'guardian')
    assert_not person.needs_carer?
  end

  # -- versioning (PaperTrail) --

  test 'creating a person records an audit trail entry' do
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

  test 'updating a person records an audit trail entry' do
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

  test 'audit trail preserves the previous value of sensitive fields' do
    PaperTrail.request.whodunnit = users(:admin).id
    john = people(:john)
    original_dob = john.date_of_birth
    john.update!(date_of_birth: 27.years.ago.to_date)
    reified = john.versions.last.reify
    assert_equal original_dob, reified.date_of_birth
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'audit trail records which user made the change' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    john = people(:john)
    john.update!(name: 'John Modified')
    assert_equal admin.id.to_s, john.versions.last.whodunnit
  ensure
    PaperTrail.request.whodunnit = nil
  end
end
