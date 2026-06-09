# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::PeopleResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

  subject(:results) { described_class.new(user: user, query: query, limit: limit, builder: builder).call }

  let(:limit) { 10 }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: query) }

  describe '#call' do
    context 'when the query matches a person name' do
      let(:user) { users(:damacus) }
      let(:query) { 'John Doe' }

      it 'returns a result for the matching person' do
        expect(results.map(&:title)).to include('John Doe')
      end

      it 'returns results with the correct type, subtitle and path' do
        result = results.find { |r| r.title == 'John Doe' }

        expect(result).to have_attributes(
          type: 'person',
          subtitle: I18n.t('global_search.types.person'),
          path: Rails.application.routes.url_helpers.person_path(people(:john)),
          score: a_kind_of(Integer)
        )
      end

      it 'orders results alphabetically by name' do
        names = results.map(&:title)

        expect(names).to eq(names.sort)
      end
    end

    context 'with a partial match' do
      let(:user) { users(:damacus) }
      let(:query) { 'Doe' }

      it 'finds all people whose name contains the query' do
        titles = results.map(&:title)

        expect(titles).to include('John Doe', 'Jane Doe')
      end
    end

    context 'when the query does not match any person' do
      let(:user) { users(:damacus) }
      let(:query) { 'ZZZNoMatch' }

      it 'returns an empty array' do
        expect(results).to be_empty
      end
    end

    context 'when policy scope restricts visible people' do
      let(:user) { users(:jane) }
      let(:query) { 'John Doe' }

      it 'does not return people outside the policy scope' do
        # jane is a parent and can only see her own person and dependents, not John
        expect(results.map(&:title)).not_to include('John Doe')
      end
    end

    context 'when results exceed the limit' do
      let(:user) { users(:damacus) }
      let(:query) { 'a' }
      let(:limit) { 2 }

      it 'returns at most limit results' do
        expect(results.size).to be <= limit
      end
    end
  end
end
