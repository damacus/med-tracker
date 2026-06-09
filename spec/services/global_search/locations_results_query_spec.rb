# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::LocationsResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:damacus) }
  let(:limit) { 10 }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: query) }

  subject(:results) { described_class.new(user: user, query: query, limit: limit, builder: builder).call }

  describe '#call' do
    context 'when the query matches a location name' do
      let(:query) { 'Home' }

      it 'returns a result for the matching location' do
        expect(results.map(&:title)).to include('Home')
      end

      it 'returns results with the correct type, subtitle and path' do
        result = results.find { |r| r.title == 'Home' }

        expect(result).to have_attributes(
          type: 'location',
          subtitle: I18n.t('global_search.types.location'),
          path: Rails.application.routes.url_helpers.location_path(locations(:home)),
          score: a_kind_of(Integer)
        )
      end

      it 'orders results alphabetically by name' do
        names = results.map(&:title)

        expect(names).to eq(names.sort)
      end
    end

    context 'when the query does not match any location name' do
      let(:query) { 'ZZZNoMatch' }

      it 'returns an empty array' do
        expect(results).to be_empty
      end
    end

    context 'with a case-insensitive partial match' do
      let(:query) { 'grandma' }

      it 'finds the location using a case-insensitive ILIKE' do
        expect(results.map(&:title)).to include("Grandma's House")
      end
    end

    context 'when results exceed the limit' do
      let(:query) { 'a' }
      let(:limit) { 1 }

      it 'returns at most limit results' do
        expect(results.size).to be <= limit
      end
    end

    context 'when the user has restricted location access' do
      let(:user) { users(:jane) }
      let(:query) { 'Home' }

      it 'returns only locations the user has access to via policy scope' do
        # jane has home and school memberships — Home should appear for her
        expect(results.map(&:title)).to include('Home')
      end

      it 'does not return a location the user has no membership for (Grandma\'s House)' do
        # jane has no membership for Grandma's House
        expect(results.map(&:title)).not_to include("Grandma's House")
      end
    end
  end
end
