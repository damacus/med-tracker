# frozen_string_literal: true

require 'test_helper'

class UserTest < ActiveSupport::TestCase
  # -- associations --

  test 'belongs_to person' do
    user = users(:john)
    assert_instance_of Person, user.person
  end

  test 'has_many prescriptions through person' do
    user = users(:john)
    assert_respond_to user, :prescriptions
  end

  # -- validations --

  test 'requires email_address' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    user = User.new(email_address: nil, password: 'password', person: person)
    assert_not user.valid?
    assert_includes user.errors[:email_address], "can't be blank"
  end

  test 'validates email uniqueness case-insensitively' do
    person = Person.new(name: 'New Person', date_of_birth: 25.years.ago)
    user = User.new(email_address: 'JOHN.DOE@EXAMPLE.COM', password: 'password', person: person)
    assert_not user.valid?
    assert_includes user.errors[:email_address], 'has already been taken'
  end

  test 'allows valid email format' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    user = User.new(email_address: 'valid@example.com', password: 'password', person: person)
    assert user.valid?
  end

  test 'rejects invalid email without TLD' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    user = User.new(email_address: 'user@example', password: 'password', person: person)
    assert_not user.valid?
  end

  test 'rejects invalid email without @' do
    person = Person.new(name: 'Test', date_of_birth: 25.years.ago)
    user = User.new(email_address: 'userexample.com', password: 'password', person: person)
    assert_not user.valid?
  end

  # -- person linkage --

  test 'requires an associated person' do
    user = User.new(email_address: 'test@example.com', password: 'password', person: nil)
    assert_not user.valid?
    assert_includes user.errors[:person], 'must exist'
  end

  # -- security --

  test 'has_secure_password' do
    person = Person.create!(name: 'Secure Test', date_of_birth: 25.years.ago)
    user = User.create!(email_address: 'secure@example.com', password: 'password', person: person)
    assert user.authenticate('password')
    assert_not user.authenticate('wrong')
  end

  # -- roles --

  test 'defines role enum with correct values' do
    assert_equal({ 'administrator' => 0, 'doctor' => 1, 'nurse' => 2, 'carer' => 3, 'parent' => 4, 'minor' => 5 },
                 User.roles)
  end

  test 'can be an administrator' do
    user = users(:admin)
    assert user.administrator?
  end

  test 'can be a doctor' do
    user = users(:doctor)
    assert user.doctor?
  end

  test 'can be a nurse' do
    user = users(:nurse)
    assert user.nurse?
  end

  test 'can be a carer' do
    user = users(:carer)
    assert user.carer?
  end

  test 'can be a parent' do
    user = users(:parent)
    assert user.parent?
  end

  # -- normalization --

  test 'downcases the email address before saving' do
    person = Person.create!(name: 'Normalize Test', date_of_birth: 25.years.ago)
    user = User.create!(email_address: 'NORMALIZE@EXAMPLE.COM', password: 'password', person: person)
    assert_equal 'normalize@example.com', user.email_address
  end

  # -- account activation --

  test 'deactivate! sets active to false' do
    user = users(:bob)
    assert user.active
    user.deactivate!
    assert_not user.reload.active
  end

  test 'activate! sets active to true' do
    user = users(:bob)
    user.update!(active: false)
    user.activate!
    assert user.reload.active
  end

  test 'active scope returns only active users' do
    users(:bob).deactivate!
    assert_not_includes User.active, users(:bob)
    assert_includes User.active, users(:admin)
  end

  test 'inactive scope returns only inactive users' do
    users(:bob).deactivate!
    assert_includes User.inactive, users(:bob)
    assert_not_includes User.inactive, users(:admin)
  end

  # -- versioning (PaperTrail) --

  test 'creates version on user creation' do
    PaperTrail.request.whodunnit = users(:admin).id
    person = Person.create!(name: 'New User', date_of_birth: 30.years.ago)
    assert_difference('PaperTrail::Version.count', 1) do
      User.create!(email_address: 'newuser@example.com', password: 'password', person: person)
    end
    version = PaperTrail::Version.last
    assert_equal 'create', version.event
    assert_equal 'User', version.item_type
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'creates version on user update' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    assert_difference('PaperTrail::Version.count', 1) do
      admin.update!(email_address: 'updated@example.com')
    end
    version = admin.versions.last
    assert_equal 'update', version.event
    assert version.object.present?
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'associates version with current user' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    admin.update!(email_address: 'uniquetest@example.com')
    assert_equal admin.id.to_s, admin.versions.last.whodunnit
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'does not track password changes' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    initial_count = admin.versions.count
    admin.update!(password: 'newpassword')
    assert_equal initial_count, admin.versions.count
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'stores version when changes occur' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    admin.update!(email_address: 'iptest@example.com')
    version = admin.versions.last
    assert version.present?
    assert_equal admin, version.item
  ensure
    PaperTrail.request.whodunnit = nil
  end
end
