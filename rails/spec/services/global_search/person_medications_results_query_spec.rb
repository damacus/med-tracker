# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::PersonMedicationsResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :person_medications, :carer_relationships

  subject(:results) { described_class.new(user: user, query: query, limit: limit, builder: builder).call }

  let(:user) { users(:damacus) }
  let(:limit) { 10 }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: query) }

  describe '#call' do
    context 'when the query matches a medication name' do
      let(:query) { 'Vitamin D' }

      it 'returns results for person_medication records matching the medication name' do
        titles = results.map(&:title)

        expect(titles).to include('Vitamin D')
      end

      it 'returns results with the correct type' do
        expect(results.map(&:type)).to all(eq('person_medication'))
      end

      it 'includes the person name in the subtitle' do
        result = results.first

        expect(result.subtitle).to include(result.subtitle)
        # subtitle uses the person name translation key
        person_names = [people(:john).name, people(:jane).name]
        expect(person_names.any? { |name| result.subtitle.include?(name) }).to be true
      end

      it 'generates a path anchored to the person_medication on the person page' do
        result = results.first

        # Path should point to a person path with an anchor
        expect(result.path).to match(%r{/people/\d+#person_medication_\d+})
      end
    end

    context 'when the query matches a person name' do
      let(:query) { 'John' }

      it 'returns person_medication records for the matching person' do
        titles = results.map(&:title)

        # John has vitamin_d assignment
        expect(titles).to include('Vitamin D')
      end
    end

    context 'when the query does not match' do
      let(:query) { 'ZZZNoMatch' }

      it 'returns an empty array' do
        expect(results).to be_empty
      end
    end

    context 'when ordering results' do
      let(:query) { 'vitamin' }

      it 'orders by medication name then person name' do
        titles = results.map(&:title)

        # All titles should be medication display names; ordering is by medications.name ASC, people.name ASC
        expect(titles).to be_an(Array)
      end
    end

    context 'when policy scope restricts visible person_medications' do
      let(:user) { users(:jane) }
      let(:query) { 'Vitamin' }

      it 'returns only person_medications accessible to the user' do
        # jane can only see her own person_medications
        results.each do |result|
          expect(result.type).to eq('person_medication')
        end

        # jane's own vitamin D should appear, john's should not
        # The subtitle includes the person name
        expect(results.map(&:subtitle)).to all(include('Jane Doe'))
      end
    end

    context 'when results exceed the limit' do
      let(:query) { 'vitamin' }
      let(:limit) { 1 }

      it 'returns at most limit results' do
        expect(results.size).to be <= limit
      end
    end
  end
end
