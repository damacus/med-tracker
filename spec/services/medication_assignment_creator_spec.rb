# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAssignmentCreator do
  describe '#call' do
    let(:person) { create(:person) }

    it 'creates a routine person medication for vitamins' do
      medication = create(:medication, category: 'Vitamin', default_schedule_type: :daily)
      dosage = create(:dosage, medication: medication, amount: 1000, unit: 'IU', default_max_daily_doses: 1,
                               default_min_hours_between_doses: 24)
      schedule_count = Schedule.count

      expect do
        assert_person_medication_result(call_creator(medication, dosage), :routine)
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      assert_person_medication_attributes(
        medication, dosage, dose_amount: '1000.0', dose_unit: 'IU', max_daily_doses: 1
      )
    end

    it 'creates an as-needed person medication for PRN medication metadata' do
      medication = create(:medication, category: 'Analgesic', default_schedule_type: :prn)
      dosage = create(:dosage, medication: medication, amount: 250, unit: 'mg', default_max_daily_doses: 4,
                               default_min_hours_between_doses: 4)
      schedule_count = Schedule.count

      expect do
        assert_person_medication_result(call_creator(medication, dosage), :as_needed)
      end.to change(PersonMedication, :count).by(1)

      expect(Schedule.count).to eq(schedule_count)
      assert_person_medication_attributes(
        medication, dosage, dose_amount: '250.0', dose_unit: 'mg',
                            max_daily_doses: 4, min_hours_between_doses: 4
      )
    end

    it 'keeps prescribed scheduled medication as a schedule' do
      medication = create(:medication, category: 'Analgesic', default_schedule_type: :multiple_daily,
                                       default_schedule_config: { 'times' => %w[08:00 20:00] })
      dosage = create(:dosage, medication: medication, amount: 200, unit: 'mg', frequency: 'Twice daily',
                               default_max_daily_doses: 2, default_min_hours_between_doses: 8)
      person_medication_count = PersonMedication.count

      expect do
        assert_schedule_result(call_creator(medication, dosage))
      end.to change(Schedule, :count).by(1)

      expect(PersonMedication.count).to eq(person_medication_count)
      assert_schedule_attributes(medication, dosage)
    end

    it 'looks up the selected predefined dose once' do
      medication = create(:medication, category: 'Vitamin', default_schedule_type: :daily)
      dosage = create(:dosage, medication: medication, amount: 1000, unit: 'IU', default_max_daily_doses: 1,
                               default_min_hours_between_doses: 24)

      expect(count_dosage_option_queries { call_creator(medication, dosage) }).to eq(1)
    end

    def call_creator(medication, dosage)
      assignment = MedicationAssignment.new(medication_id: medication.id, source_dosage_option_id: dosage.id)
      described_class.new(person: person, medication_scope: Medication.all, assignment: assignment).call
    end

    def count_dosage_option_queries(&)
      count = 0

      subscriber = lambda do |_name, _start, _finish, _id, payload|
        sql = payload[:sql]
        next if payload[:cached] || payload[:name] == 'SCHEMA'

        count += 1 if sql.include?('FROM "dosages"') && sql.include?('"dosages"."id"')
      end

      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
      count
    end

    def assert_person_medication_result(result, kind)
      expect(result.success).to be(true)
      expect(result.record).to be_a(PersonMedication)
      expect(result.schedule).to be_nil
      expect(result.record.public_send("#{kind}?")).to be(true)
    end

    def assert_schedule_result(result)
      expect(result.success).to be(true)
      expect(result.record).to be_a(Schedule)
      expect(result.schedule).to eq(result.record)
    end

    def assert_person_medication_attributes(medication, dosage, attributes)
      person_medication = PersonMedication.order(:id).last
      expect(person_medication).to have_attributes(
        attributes.merge(
          person: person,
          medication: medication,
          source_dosage_option: dosage,
          dose_amount: BigDecimal(attributes[:dose_amount])
        )
      )
      expect(person_medication.dose_cycle).to eq('daily')
    end

    def assert_schedule_attributes(medication, dosage)
      schedule = Schedule.order(:id).last
      expect(schedule).to have_attributes(
        person: person,
        medication: medication,
        source_dosage_option: dosage,
        dose_amount: BigDecimal('200.0'),
        dose_unit: 'mg',
        frequency: 'Twice daily',
        max_daily_doses: 2,
        min_hours_between_doses: 8
      )
      expect(schedule.schedule_type).to eq('multiple_daily')
    end
  end
end
