# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Context do
  subject(:context) do
    described_class.new(people: [person], start_date: start_date, end_date: end_date)
  end

  let(:person)     { create(:person) }
  let(:start_date) { 14.days.ago.to_date }
  let(:end_date)   { Time.zone.today }

  describe '#initialize' do
    it 'stores people, start_date, and end_date' do
      expect(context.people).to eq([person])
      expect(context.start_date).to eq(start_date)
      expect(context.end_date).to eq(end_date)
    end
  end

  describe '#evidence_days' do
    it 'counts the number of days in the date range (inclusive)' do
      ctx = described_class.new(people: [person],
                                start_date: Time.zone.today - 6,
                                end_date: Time.zone.today)
      expect(ctx.evidence_days).to eq(7)
    end
  end

  describe '#schedules' do
    it 'returns schedules for the given people that overlap the date range' do
      medication = create(:medication)
      schedule = create(:schedule, person: person, medication: medication,
                                   start_date: start_date, end_date: end_date)
      expect(context.schedules).to include(schedule)
    end

    it 'excludes schedules for other people' do
      other_person = create(:person)
      medication   = create(:medication)
      other_schedule = create(:schedule, person: other_person, medication: medication,
                                         start_date: start_date, end_date: end_date)
      expect(context.schedules).not_to include(other_schedule)
    end

    it 'excludes schedules that ended before start_date' do
      medication = create(:medication)
      old_schedule = create(:schedule, person: person, medication: medication,
                                       start_date: 1.year.ago.to_date,
                                       end_date: start_date - 1.day)
      expect(context.schedules).not_to include(old_schedule)
    end

    it 'is memoized' do
      # Warm the cache, then verify no additional DB call is made
      context.schedules
      allow(Schedule).to receive(:where).and_call_original
      context.schedules
      expect(Schedule).not_to have_received(:where)
    end
  end

  describe '#active_schedules' do
    it 'returns only schedules that are active today' do
      medication = create(:medication)
      active    = create(:schedule, person: person, medication: medication,
                                    start_date: 7.days.ago.to_date, end_date: 1.year.from_now.to_date)
      expired   = create(:schedule, person: person, medication: medication,
                                    start_date: start_date, end_date: 1.day.ago.to_date)
      expect(context.active_schedules).to include(active)
      expect(context.active_schedules).not_to include(expired)
    end
  end

  describe '#person_medications' do
    it 'returns person medications for the given people' do
      pm = create(:person_medication, person: person)
      expect(context.person_medications).to include(pm)
    end

    it 'excludes person medications for other people' do
      other = create(:person)
      pm    = create(:person_medication, person: other)
      expect(context.person_medications).not_to include(pm)
    end
  end

  describe '#prn_sources' do
    it 'includes prn-type schedules' do
      medication = create(:medication)
      prn_sched = create(:schedule, person: person, medication: medication,
                                    start_date: start_date, end_date: end_date,
                                    schedule_type: :prn)
      expect(context.prn_sources).to include(prn_sched)
    end

    it 'includes as-needed person medications' do
      pm = create(:person_medication, :as_needed, person: person)
      expect(context.prn_sources).to include(pm)
    end
  end

  describe '#enough_evidence?' do
    it 'returns false when the window is shorter than MINIMUM_EVIDENCE_DAYS' do
      ctx = described_class.new(people: [person],
                                start_date: Time.zone.today - 3,
                                end_date: Time.zone.today)
      expect(ctx.enough_evidence?).to be(false)
    end
  end

  describe '#evidence_summary' do
    it 'returns a translated string (does not raise)' do
      expect { context.evidence_summary }.not_to raise_error
      expect(context.evidence_summary).to be_a(String)
    end
  end

  describe 'people as ActiveRecord::Relation' do
    it 'accepts an AR relation for people (uses pluck)' do
      relation = Person.where(id: person.id)
      ctx = described_class.new(people: relation, start_date: start_date, end_date: end_date)
      expect { ctx.schedules }.not_to raise_error
    end
  end
end
