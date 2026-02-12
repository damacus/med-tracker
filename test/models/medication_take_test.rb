# frozen_string_literal: true

require 'test_helper'

class MedicationTakeTest < ActiveSupport::TestCase
  setup do
    @person = Person.create!(name: 'Jane Doe', date_of_birth: '1990-01-01')
    @medicine = Medicine.create!(name: 'Lisinopril', current_supply: 50, stock: 50, reorder_threshold: 10)
    @dosage = Dosage.create!(medicine: @medicine, amount: 10, unit: 'mg', frequency: 'daily')
    @prescription = Prescription.create!(
      person: @person, medicine: @medicine, dosage: @dosage,
      start_date: Time.zone.today, end_date: Time.zone.today + 30.days
    )
    @person_medicine = PersonMedicine.create!(person: @person, medicine: @medicine, notes: 'Test notes')
  end

  # -- validations --

  test 'requires taken_at' do
    take = MedicationTake.new(prescription: @prescription)
    assert_not take.valid?
    assert_includes take.errors[:taken_at], "can't be blank"
  end

  # -- associations --

  test 'belongs_to prescription optionally' do
    take = MedicationTake.new(prescription: @prescription, taken_at: Time.current)
    assert take.valid?
    assert_instance_of Prescription, take.prescription
  end

  test 'belongs_to person_medicine optionally' do
    take = MedicationTake.new(person_medicine: @person_medicine, taken_at: Time.current)
    assert take.valid?
    assert_instance_of PersonMedicine, take.person_medicine
  end

  # -- source validation --

  test 'invalid when neither prescription nor person_medicine is set' do
    take = MedicationTake.new(taken_at: Time.current)
    assert_not take.valid?
    assert_includes take.errors[:base], 'Must have exactly one source (prescription or person_medicine)'
  end

  test 'invalid when both prescription and person_medicine are set' do
    take = MedicationTake.new(
      prescription: @prescription,
      person_medicine: @person_medicine,
      taken_at: Time.current
    )
    assert_not take.valid?
    assert_includes take.errors[:base], 'Must have exactly one source (prescription or person_medicine)'
  end

  test 'valid when only prescription is set' do
    take = MedicationTake.new(prescription: @prescription, taken_at: Time.current)
    assert take.valid?
  end

  test 'valid when only person_medicine is set' do
    take = MedicationTake.new(person_medicine: @person_medicine, taken_at: Time.current)
    assert take.valid?
  end

  # -- stock tracking --

  test 'deducts 1 from medicine stock when taking from prescription' do
    @medicine.update!(stock: 100, current_supply: 100)
    assert_difference -> { @medicine.reload.stock }, -1 do
      MedicationTake.create!(prescription: @prescription, taken_at: Time.current)
    end
  end

  test 'deducts 1 from medicine current_supply when taking from prescription' do
    @medicine.update!(stock: 100, current_supply: 100)
    assert_difference -> { @medicine.reload.current_supply }, -1 do
      MedicationTake.create!(prescription: @prescription, taken_at: Time.current)
    end
  end

  test 'deducts 1 from medicine stock when taking from person_medicine' do
    @medicine.update!(stock: 100, current_supply: 100)
    assert_difference -> { @medicine.reload.stock }, -1 do
      MedicationTake.create!(person_medicine: @person_medicine, taken_at: Time.current)
    end
  end

  # -- versioning (PaperTrail) --

  test 'creates version when medication is taken' do
    PaperTrail.request.whodunnit = users(:admin).id
    assert_difference('PaperTrail::Version.count', 1) do
      MedicationTake.create!(prescription: @prescription, taken_at: Time.current, amount_ml: 5.0)
    end
    version = PaperTrail::Version.last
    assert_equal 'create', version.event
    assert_equal 'MedicationTake', version.item_type
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'creates version on medication take update' do
    PaperTrail.request.whodunnit = users(:admin).id
    take = MedicationTake.create!(prescription: @prescription, taken_at: Time.current, amount_ml: 5.0)
    assert_difference('PaperTrail::Version.count', 1) do
      take.update!(amount_ml: 10.0)
    end
    version = take.versions.last
    assert_equal 'update', version.event
    assert version.object.present?
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'tracks time changes' do
    PaperTrail.request.whodunnit = users(:admin).id
    original_time = 2.hours.ago
    take = MedicationTake.create!(prescription: @prescription, taken_at: original_time, amount_ml: 5.0)
    take.update!(taken_at: 1.hour.ago)
    reified = take.versions.last.reify
    assert_equal original_time.to_i, reified.taken_at.to_i
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'associates version with current user' do
    admin = users(:admin)
    PaperTrail.request.whodunnit = admin.id
    take = MedicationTake.create!(prescription: @prescription, taken_at: Time.current, amount_ml: 5.0)
    assert_equal admin.id.to_s, take.versions.last.whodunnit
  ensure
    PaperTrail.request.whodunnit = nil
  end

  test 'records IP address when controller_info is set' do
    PaperTrail.request.whodunnit = users(:admin).id
    PaperTrail.request.controller_info = { ip: '192.168.1.100' }
    take = MedicationTake.create!(prescription: @prescription, taken_at: Time.current, amount_ml: 5.0)
    assert_equal '192.168.1.100', take.versions.last.ip
  ensure
    PaperTrail.request.whodunnit = nil
    PaperTrail.request.controller_info = nil
  end
end
