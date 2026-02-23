# frozen_string_literal: true

require 'rails_helper'

RSpec.describe FamilyDashboard::ScheduleQuery do
  fixtures :people, :carer_relationships, :prescriptions, :person_medicines, :medication_takes,
           :locations, :medicines, :dosages

  let(:jane) { people(:jane) }
  let(:child) { people(:child_patient) }
  let(:query) { described_class.new([jane, child]) }

  describe '#call' do
    it 'returns an aggregated list of doses for the person and their dependents' do
      results = query.call
      expect(results).to be_an(Array)

      # Jane and her child should be in the results
      people_in_results = results.pluck(:person).uniq
      expect(people_in_results).to contain_exactly(jane, child)
    end

    it 'includes prescription doses from both the parent and the child' do
      results = query.call
      prescriptions = results.pluck(:source).grep(Prescription)

      expect(prescriptions).to include(prescriptions(:jane_ibuprofen))
      expect(prescriptions).to include(prescriptions(:patient_prescription)) # child_patient's ibuprofen
    end

    it 'includes person_medicine (non-prescription) doses' do
      results = query.call
      pms = results.pluck(:source).grep(PersonMedicine)
      expect(pms).to include(person_medicines(:jane_vitamin_d))
    end

    it 'returns doses sorted by scheduled_at' do
      results = query.call
      times = results.pluck(:scheduled_at)
      expect(times).to eq(times.sort)
    end

    it 'correctly identifies doses already taken today' do
      results = query.call
      taken_doses = results.select { |r| r[:status] == :taken }

      # Jane has jane_evening_ibuprofen and jane_morning_ibuprofen in fixtures
      jane_takes = taken_doses.select { |r| r[:person] == jane }
      expect(jane_takes).not_to be_empty
      expect(jane_takes.first[:taken_at]).to be_present
    end

    it 'filters out inactive prescriptions' do
      # Create an inactive prescription for Jane
      Prescription.create!(
        person: jane,
        medicine: medicines(:paracetamol),
        dosage: dosages(:paracetamol_adult),
        start_date: 1.year.ago,
        end_date: 1.month.ago,
        frequency: 'Daily'
      )

      results = query.call
      active_prescriptions = results.pluck(:source).grep(Prescription)
      active_prescriptions.each do |p|
        expect(p.start_date).to be <= Time.zone.today
        expect(p.end_date).to be >= Time.zone.today
      end
    end

    context 'when a prescription has no timing restrictions' do
      let!(:prescription) do
        Prescription.create!(
          person: people(:john),
          medicine: medicines(:paracetamol),
          dosage: dosages(:paracetamol_adult),
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'As needed',
          max_daily_doses: nil,
          min_hours_between_doses: nil
        )
      end

      it 'does not emit an upcoming row for a source with no timing restrictions' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == prescription && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when a prescription is on cooldown' do
      let!(:prescription) do
        Prescription.create!(
          person: people(:john),
          medicine: medicines(:ibuprofen),
          dosage: dosages(:ibuprofen_adult),
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'Every 2 hours',
          min_hours_between_doses: 2,
          max_daily_doses: nil
        )
      end

      before do
        MedicationTake.create!(prescription: prescription, taken_at: 1.hour.ago, amount_ml: 400)
      end

      it 'emits a row with :cooldown status' do
        results = described_class.new([people(:john)]).call
        cooldown_rows = results.select { |r| r[:source] == prescription && r[:status] == :cooldown }
        expect(cooldown_rows).not_to be_empty
      end

      it 'does not emit an :upcoming row for the same source' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == prescription && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when max daily doses are reached' do
      let!(:prescription) do
        Prescription.create!(
          person: people(:john),
          medicine: medicines(:paracetamol),
          dosage: dosages(:paracetamol_adult),
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'As needed',
          max_daily_doses: 2
        )
      end

      before do
        2.times { MedicationTake.create!(prescription: prescription, taken_at: Time.current, amount_ml: 500) }
      end

      it 'does not emit an :upcoming row when daily max is reached' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == prescription && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end

    context 'when medicine is out of stock' do
      let!(:prescription) do
        oos_medicine = Medicine.create!(name: 'OOS Med', current_supply: 0, stock: 10, reorder_threshold: 2)
        dosage = Dosage.create!(medicine: oos_medicine, amount: 10, unit: 'mg', frequency: 'daily')
        Prescription.create!(
          person: people(:john),
          medicine: oos_medicine,
          dosage: dosage,
          start_date: Time.zone.today,
          end_date: 1.year.from_now.to_date,
          frequency: 'daily',
          min_hours_between_doses: 4
        )
      end

      it 'emits a row with :out_of_stock status' do
        results = described_class.new([people(:john)]).call
        oos_rows = results.select { |r| r[:source] == prescription && r[:status] == :out_of_stock }
        expect(oos_rows).not_to be_empty
      end

      it 'does not emit an :upcoming row when out of stock' do
        results = described_class.new([people(:john)]).call
        upcoming = results.select { |r| r[:source] == prescription && r[:status] == :upcoming }
        expect(upcoming).to be_empty
      end
    end
  end
end
