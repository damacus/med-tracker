# frozen_string_literal: true

require 'test_helper'

class PrescriptionTest < ActiveSupport::TestCase
  # -- associations --

  test 'belongs_to person' do
    prescription = prescriptions(:john_paracetamol)
    assert_instance_of Person, prescription.person
  end

  test 'belongs_to dosage' do
    prescription = prescriptions(:john_paracetamol)
    assert_instance_of Dosage, prescription.dosage
  end

  test 'has_many medication_takes' do
    prescription = prescriptions(:john_paracetamol)
    assert_respond_to prescription, :medication_takes
    assert_kind_of ActiveRecord::Associations::CollectionProxy, prescription.medication_takes
  end

  # -- active flag --

  test 'is active by default' do
    prescription = Prescription.new(
      person: people(:john),
      medicine: medicines(:paracetamol),
      dosage: dosages(:paracetamol_adult),
      start_date: Time.zone.today,
      end_date: Time.zone.today + 30.days
    )
    prescription.save!
    assert prescription.active
  end

  test 'can be set to inactive' do
    prescription = prescriptions(:john_paracetamol)
    prescription.update!(active: false)
    assert_not prescription.active
  end

  test 'can be reactivated' do
    prescription = prescriptions(:john_paracetamol)
    prescription.update!(active: false)
    prescription.update!(active: true)
    assert prescription.active
  end

  # -- validations --

  test 'requires start_date' do
    prescription = Prescription.new(
      person: people(:john),
      medicine: medicines(:paracetamol),
      dosage: dosages(:paracetamol_adult),
      end_date: Time.zone.today + 30.days
    )
    assert_not prescription.valid?
    assert_includes prescription.errors[:start_date], "can't be blank"
  end

  test 'requires end_date' do
    prescription = Prescription.new(
      person: people(:john),
      medicine: medicines(:paracetamol),
      dosage: dosages(:paracetamol_adult),
      start_date: Time.zone.today
    )
    assert_not prescription.valid?
    assert_includes prescription.errors[:end_date], "can't be blank"
  end

  test 'invalid if end_date is before start_date' do
    prescription = Prescription.new(
      person: people(:john),
      medicine: medicines(:paracetamol),
      dosage: dosages(:paracetamol_adult),
      start_date: Time.zone.today,
      end_date: Time.zone.today - 1.day
    )
    assert_not prescription.valid?
    assert_includes prescription.errors[:end_date], 'must be after the start date'
  end

  # -- #can_administer? --

  test 'can_administer? returns true when can take and medicine in stock' do
    medicine = Medicine.create!(name: 'TestMed', stock: 10, reorder_threshold: 2)
    person = Person.create!(name: 'Test', email: 'test-administer@example.com', date_of_birth: Date.new(1990, 1, 1))
    dosage = Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily')
    prescription = Prescription.create!(
      person: person, medicine: medicine, dosage: dosage,
      start_date: Time.zone.today, end_date: Time.zone.today + 30.days
    )
    assert prescription.can_administer?
  end

  test 'can_administer? returns false when medicine out of stock' do
    medicine = Medicine.create!(name: 'TestMed', stock: 0, reorder_threshold: 2)
    person = Person.create!(name: 'Test', email: 'test-oos@example.com', date_of_birth: Date.new(1990, 1, 1))
    dosage = Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily')
    prescription = Prescription.create!(
      person: person, medicine: medicine, dosage: dosage,
      start_date: Time.zone.today, end_date: Time.zone.today + 30.days
    )
    assert_not prescription.can_administer?
  end

  test 'can_administer? returns true when stock is nil (untracked)' do
    medicine = Medicine.create!(name: 'TestMed', stock: nil, reorder_threshold: 2)
    person = Person.create!(name: 'Test', email: 'test-nil@example.com', date_of_birth: Date.new(1990, 1, 1))
    dosage = Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily')
    prescription = Prescription.create!(
      person: person, medicine: medicine, dosage: dosage,
      start_date: Time.zone.today, end_date: Time.zone.today + 30.days
    )
    assert prescription.can_administer?
  end

  # -- #administration_blocked_reason --

  test 'administration_blocked_reason returns :out_of_stock when no stock' do
    medicine = Medicine.create!(name: 'TestMed2', stock: 0, reorder_threshold: 2)
    person = Person.create!(name: 'Test', email: 'test-reason@example.com', date_of_birth: Date.new(1990, 1, 1))
    dosage = Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily')
    prescription = Prescription.create!(
      person: person, medicine: medicine, dosage: dosage,
      start_date: Time.zone.today, end_date: Time.zone.today + 30.days
    )
    assert_equal :out_of_stock, prescription.administration_blocked_reason
  end
end
