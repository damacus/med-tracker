# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::RecordResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:damacus) }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: 'home') }

  describe '#initialize' do
    it 'exposes the constructor arguments as attributes' do
      query = described_class.new(user: user, query: 'home', limit: 5, builder: builder)

      expect(query).to have_attributes(user: user, query: 'home', limit: 5, builder: builder)
    end
  end

  describe '#scoped (via subclass)' do
    # RecordResultsQuery is an abstract base — test scoping via a concrete subclass
    it 'applies the Pundit policy scope to the model' do
      admin_user = users(:damacus)
      query = GlobalSearch::LocationsResultsQuery.new(user: admin_user, query: 'home', limit: 10, builder: builder)

      # Admin sees all locations
      expect(query.call).not_to be_empty
    end

    it 'returns only locations accessible to the user per policy scope' do
      # jane is a parent — she can only see locations linked to herself or her dependents
      jane = users(:jane)
      jane_builder = GlobalSearch::ResultBuilder.new(query: 'school')
      jane_query = GlobalSearch::LocationsResultsQuery.new(
        user: jane, query: 'school', limit: 10, builder: jane_builder
      )

      # jane_school membership exists so Jane can see the School location
      expect(jane_query.call.map(&:title)).to include('School')
    end
  end
end
