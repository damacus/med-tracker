# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPlanClassifier do
  let(:medication) { instance_double(Medication, category: 'Analgesic', default_schedule_type: nil) }

  describe '#direct?' do
    it 'is true for the "vitamin" category' do
      med = instance_double(Medication, category: 'Vitamin', default_schedule_type: nil)
      expect(described_class.new(medication: med).direct?).to be(true)
    end

    it 'is true for the "supplement" category' do
      med = instance_double(Medication, category: 'Supplement', default_schedule_type: nil)
      expect(described_class.new(medication: med).direct?).to be(true)
    end

    it 'is true for the "mineral" category' do
      med = instance_double(Medication, category: 'Mineral', default_schedule_type: nil)
      expect(described_class.new(medication: med).direct?).to be(true)
    end

    it 'is true for supplement categories regardless of case' do
      med = instance_double(Medication, category: 'VITAMIN', default_schedule_type: nil)
      expect(described_class.new(medication: med).direct?).to be(true)
    end

    it 'is true when the schedule_type is prn' do
      expect(described_class.new(medication: medication, schedule_type: 'prn').direct?).to be(true)
    end

    it 'is false for a non-supplement with a non-prn schedule_type' do
      expect(described_class.new(medication: medication, schedule_type: 'multiple_daily').direct?).to be(false)
    end

    it 'is false for a non-supplement with no schedule_type' do
      expect(described_class.new(medication: medication).direct?).to be(false)
    end

    it 'is false when category is nil' do
      med = instance_double(Medication, category: nil, default_schedule_type: nil)
      expect(described_class.new(medication: med).direct?).to be(false)
    end
  end

  describe '#administration_kind' do
    it "is 'routine' for vitamin category" do
      med = instance_double(Medication, category: 'vitamin', default_schedule_type: nil)
      expect(described_class.new(medication: med).administration_kind).to eq('routine')
    end

    it "is 'routine' for supplement category" do
      med = instance_double(Medication, category: 'supplement', default_schedule_type: nil)
      expect(described_class.new(medication: med).administration_kind).to eq('routine')
    end

    it "is 'routine' for mineral category" do
      med = instance_double(Medication, category: 'mineral', default_schedule_type: nil)
      expect(described_class.new(medication: med).administration_kind).to eq('routine')
    end

    it "is 'as_needed' for non-supplement category" do
      expect(described_class.new(medication: medication).administration_kind).to eq('as_needed')
    end

    it "is 'as_needed' even when schedule_type is 'prn' for a non-supplement" do
      expect(described_class.new(medication: medication, schedule_type: 'prn').administration_kind).to eq('as_needed')
    end
  end

  describe '#schedule_type' do
    it 'prefers the explicitly passed schedule_type' do
      expect(described_class.new(medication: medication, schedule_type: 'daily').schedule_type).to eq('daily')
    end

    it 'falls back to the medication default when none is passed' do
      med = instance_double(Medication, category: 'x', default_schedule_type: 'weekly')
      expect(described_class.new(medication: med).schedule_type).to eq('weekly')
    end

    it "falls back to 'multiple_daily' when nothing is set" do
      expect(described_class.new(medication: medication).schedule_type).to eq('multiple_daily')
    end

    it 'ignores a blank (empty string) passed schedule_type and falls back to medication default' do
      med = instance_double(Medication, category: 'x', default_schedule_type: 'weekly')
      expect(described_class.new(medication: med, schedule_type: '').schedule_type).to eq('weekly')
    end

    it "ignores a blank medication default and falls back to 'multiple_daily'" do
      med = instance_double(Medication, category: 'x', default_schedule_type: '')
      expect(described_class.new(medication: med).schedule_type).to eq('multiple_daily')
    end
  end
end
