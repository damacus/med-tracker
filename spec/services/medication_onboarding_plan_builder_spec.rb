# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationOnboardingPlanBuilder do
  let(:person)   { create(:person) }
  let(:medication) { create(:medication) }
  let(:dosage)   { create(:dosage, medication: medication, amount: 500, unit: 'mg', frequency: 'As needed') }

  def build_record(schedule_type:, extra_schedule_attrs: {}, schedule_config: {})
    schedule_attrs = {
      schedule_type: schedule_type,
      max_daily_doses: 4,
      min_hours_between_doses: 4,
      dose_cycle: 'daily'
    }.merge(extra_schedule_attrs)

    described_class.new(
      person: person,
      medication: medication,
      dosage: dosage,
      schedule_attributes: schedule_attrs,
      schedule_config: schedule_config
    ).record
  end

  describe '#record for a direct (PRN) plan' do
    subject(:record) { build_record(schedule_type: 'prn') }

    it 'builds a PersonMedication (not a Schedule)' do
      expect(record).to be_a(PersonMedication)
    end

    it 'is not persisted' do
      expect(record).to be_new_record
    end

    it 'associates with the correct person' do
      expect(record.person).to eq(person)
    end

    it 'associates with the correct medication' do
      expect(record.medication).to eq(medication)
    end

    it 'sets source_dosage_option to the supplied dosage' do
      expect(record.source_dosage_option).to eq(dosage)
    end

    it 'copies dose_amount from the dosage' do
      expect(record.dose_amount).to eq(dosage.amount)
    end

    it 'copies dose_unit from the dosage' do
      expect(record.dose_unit).to eq(dosage.unit)
    end

    it 'carries max_daily_doses from schedule_attributes' do
      expect(record.max_daily_doses).to eq(4)
    end

    it 'carries min_hours_between_doses from schedule_attributes' do
      expect(record.min_hours_between_doses).to eq(4)
    end

    it 'carries dose_cycle from schedule_attributes' do
      expect(record.dose_cycle).to eq('daily')
    end
  end

  describe '#record for a supplement (Vitamin) direct plan' do
    let(:medication) { create(:medication, :vitamin, category: 'Vitamin') }

    subject(:record) { build_record(schedule_type: 'daily') }

    it 'builds a PersonMedication for supplement category' do
      expect(record).to be_a(PersonMedication)
    end

    it 'sets administration_kind to routine for supplements' do
      expect(record.administration_kind).to eq('routine')
    end
  end

  describe '#record for a scheduled (multiple_daily) plan' do
    subject(:record) do
      build_record(
        schedule_type: 'multiple_daily',
        extra_schedule_attrs: {
          start_date: Date.today,
          end_date: 1.month.from_now.to_date,
          frequency: 'Twice daily'
        },
        schedule_config: { 'times' => %w[08:00 20:00] }
      )
    end

    it 'builds a Schedule (not a PersonMedication)' do
      expect(record).to be_a(Schedule)
    end

    it 'is not persisted' do
      expect(record).to be_new_record
    end

    it 'associates with the correct person' do
      expect(record.person).to eq(person)
    end

    it 'associates with the correct medication' do
      expect(record.medication).to eq(medication)
    end

    it 'sets source_dosage_option to the supplied dosage' do
      expect(record.source_dosage_option).to eq(dosage)
    end

    it 'copies dose_amount from the dosage' do
      expect(record.dose_amount).to eq(dosage.amount)
    end

    it 'copies dose_unit from the dosage' do
      expect(record.dose_unit).to eq(dosage.unit)
    end

    it 'uses the explicit frequency from schedule_attributes' do
      expect(record.frequency).to eq('Twice daily')
    end

    it 'carries start_date from schedule_attributes' do
      expect(record.start_date).to eq(Date.today)
    end

    it 'sets schedule_type to multiple_daily' do
      expect(record.schedule_type).to eq('multiple_daily')
    end

    it 'carries schedule_config' do
      expect(record.schedule_config).to eq('times' => %w[08:00 20:00])
    end
  end

  describe '#record frequency fallback to dosage frequency' do
    subject(:record) do
      build_record(
        schedule_type: 'multiple_daily',
        extra_schedule_attrs: { frequency: nil }
      )
    end

    it 'falls back to the dosage frequency when schedule frequency is blank' do
      expect(record.frequency).to eq(dosage.frequency)
    end
  end
end
