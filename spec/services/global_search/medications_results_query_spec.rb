# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::MedicationsResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  subject(:results) { described_class.new(user: user, query: query, limit: limit, builder: builder).call }

  let(:user) { users(:damacus) }
  let(:limit) { 10 }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: query) }

  describe '#call' do
    context 'when the query matches a medication name' do
      let(:query) { 'Vitamin D' }

      it 'returns a result for the matching medication' do
        expect(results.map(&:title)).to include('Vitamin D')
      end

      it 'returns results with the correct type and path' do
        result = results.find { |r| r.title == 'Vitamin D' }

        expect(result).to have_attributes(
          type: 'medication',
          path: Rails.application.routes.url_helpers.medication_path('test-household', medications(:vitamin_d)),
          score: a_kind_of(Integer)
        )
      end

      it 'uses the display name for the title' do
        result = results.find { |r| r.title == 'Vitamin D' }

        expect(result.title).to eq(medications(:vitamin_d).display_name)
      end
    end

    context 'when the query matches a friendly name' do
      let(:query) { 'Movicol Paediatric Plain' }

      it 'finds the medication by its friendly name' do
        # Movicol has a long name but friendly_name of "Movicol Paediatric Plain"
        titles = results.map(&:title)

        expect(titles).to include('Movicol Paediatric Plain')
      end
    end

    context 'when the query matches a medication category' do
      let(:query) { 'Vitamin' }

      it 'returns medications in the matching category' do
        titles = results.map(&:title)

        expect(titles).to include('Vitamin D', 'Vitamin C')
      end
    end

    context 'when the query does not match any medication' do
      let(:query) { 'ZZZNoMatch' }

      it 'returns an empty array' do
        expect(results).to be_empty
      end
    end

    context 'when building the subtitle' do
      let(:query) { 'Vitamin D' }

      it 'includes category and location name in the subtitle when both are present' do
        result = results.find { |r| r.title == 'Vitamin D' }
        med = medications(:vitamin_d)

        expect(result.subtitle).to include(med.category)
        expect(result.subtitle).to include(med.location.name)
      end
    end

    context 'when a medication has no category' do
      let(:query) { 'NoCat Med' }

      before do
        Medication.create!(
          name: 'NoCat Med',
          location: locations(:home),
          dosage_amount: 1,
          dosage_unit: 'mg',
          current_supply: 0,
          reorder_threshold: 0
        )
      end

      it 'uses the location name alone as the subtitle (no category separator)' do
        result = results.find { |r| r.title == 'NoCat Med' }

        expect(result.subtitle).to eq('Home')
      end
    end

    context 'when results exceed the limit' do
      let(:query) { 'a' }
      let(:limit) { 2 }

      it 'returns at most limit results' do
        expect(results.size).to be <= limit
      end
    end

    context 'when ordering results' do
      let(:query) { 'a' }

      it 'returns results ordered alphabetically by medication name' do
        titles = results.map(&:title)

        expect(titles).to eq(titles.sort)
      end
    end
  end
end
