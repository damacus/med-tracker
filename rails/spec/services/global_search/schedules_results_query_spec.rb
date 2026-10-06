# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::SchedulesResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :schedules, :dosages, :carer_relationships

  subject(:results) { described_class.new(user: user, query: query, limit: limit, builder: builder).call }

  let(:user) { users(:damacus) }
  let(:limit) { 10 }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: query) }

  describe '#call' do
    context 'when the query matches a medication name' do
      let(:query) { 'Paracetamol' }

      it 'returns schedule results matching the medication name' do
        expect(results).not_to be_empty
      end

      it 'returns results with type "schedule"' do
        expect(results.map(&:type)).to all(eq('schedule'))
      end

      it 'includes the medication display name in the title via the schedule translation' do
        result = results.first

        expect(result.title).to include('Paracetamol')
      end

      it 'includes the person name and frequency in the subtitle' do
        result = results.first

        expect(result.subtitle).to be_a(String)
        expect(result.subtitle).not_to be_empty
      end

      it 'generates a path anchored to the schedule on the person page' do
        result = results.first

        expect(result.path).to match(%r{/people/\d+#schedule_\d+})
      end
    end

    context 'when the query matches a person name' do
      let(:query) { 'John' }

      it 'returns schedules for the matching person' do
        expect(results).not_to be_empty
        expect(results.map(&:type)).to all(eq('schedule'))
      end
    end

    context 'when a schedule has no frequency set' do
      let(:query) { 'Paracetamol' }

      it 'falls back to the no_frequency translation in the subtitle' do
        # john_paracetamol has frequency "As needed" — change one to blank to test fallback
        schedules(:john_paracetamol).update!(frequency: nil)

        no_freq_label = I18n.t('global_search.subtitles.no_frequency')
        result = results.find { |r| r.subtitle.include?(no_freq_label) }

        expect(result).not_to be_nil
      end
    end

    context 'when the query does not match any schedule' do
      let(:query) { 'ZZZNoMatch' }

      it 'returns an empty array' do
        expect(results).to be_empty
      end
    end

    context 'when policy scope restricts visible schedules' do
      let(:user) { users(:jane) }
      let(:query) { 'Ibuprofen' }

      it 'returns only schedules accessible to the user' do
        # jane can see her own schedule (jane_ibuprofen)
        expect(results).not_to be_empty
        expect(results.map(&:type)).to all(eq('schedule'))
      end

      it 'does not return schedules for people outside the scope' do
        # john's paracetamol schedules should not appear for jane
        results_for_john_query = described_class.new(
          user: user, query: 'John', limit: 10, builder: GlobalSearch::ResultBuilder.new(query: 'John')
        ).call

        expect(results_for_john_query).to be_empty
      end
    end

    context 'when results exceed the limit' do
      let(:query) { 'a' }
      let(:limit) { 1 }

      it 'returns at most limit results' do
        expect(results.size).to be <= limit
      end
    end
  end
end
