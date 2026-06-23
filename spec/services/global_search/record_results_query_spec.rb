# frozen_string_literal: true

require 'rails_helper'

RSpec.describe GlobalSearch::RecordResultsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:user) { users(:damacus) }
  let(:builder) { GlobalSearch::ResultBuilder.new(query: 'home') }
  let(:helper_class) do
    Class.new(described_class) do
      public :default_url_options, :household_slug_for, :record_household, :tenant_route_args, :tenant_route_options,
             :user_household_slug
    end
  end

  def helper_query(user:)
    helper_class.new(user: user, query: 'home', limit: 5, builder: builder)
  end

  def household_for(name)
    Household.create!(name: name, slug: name.parameterize)
  end

  def person_for(household)
    household.people.create!(
      name: "#{household.name} Person",
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def user_for(household)
    Struct.new(:person).new(person_for(household))
  end

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

  describe 'tenant route helpers' do
    it 'uses the current household for default URL options when present' do
      household = household_for('Current Search Household')
      Current.household = household

      expect(helper_query(user: user).default_url_options).to eq(household_slug: household.slug)
    ensure
      Current.reset
    end

    it 'falls back to the user person household for default URL options' do
      Current.household = nil
      household = household_for('Person Search Household')

      expect(helper_query(user: user_for(household)).default_url_options).to eq(household_slug: household.slug)
    end

    it 'falls back to the user account household for default URL options' do
      Current.household = nil
      household = household_for('Account Search Household')
      account = Account.create!(email: 'search-account@example.test', status: :verified)
      person = Person.create!(
        account: account,
        name: 'Search Account Person',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      )
      household.household_memberships.create!(account: account, role: :member, status: :active)
      search_user = Struct.new(:person).new(person)

      expect(helper_query(user: search_user).default_url_options).to eq(household_slug: household.slug)
    end

    it 'returns empty default URL options without a resolvable household' do
      Current.household = nil
      search_user = Struct.new(:person).new(nil)

      expect(helper_query(user: search_user).default_url_options).to eq({})
    end

    it 'resolves household slugs from current, record, and user households' do
      current_household = household_for('Current Route Household')
      record_household = household_for('Record Route Household')
      user_household = household_for('User Route Household')
      record = Location.create!(name: 'Record Route Location', household: record_household)
      query = helper_query(user: user_for(user_household))

      Current.household = current_household
      expect(query.household_slug_for(record)).to eq(current_household.slug)
      Current.household = nil

      expect(query.household_slug_for(record)).to eq(record_household.slug)
      expect(query.household_slug_for(Object.new)).to eq(user_household.slug)
    ensure
      Current.reset
    end

    it 'detects record households from household and person associations' do
      household = household_for('Record Association Household')
      query = helper_query(user: user)
      location = Location.create!(name: 'Association Route Location', household: household)
      person_record = Struct.new(:person).new(person_for(household))

      expect(query.record_household(location)).to eq(household)
      expect(query.record_household(person_record)).to eq(household)
    end

    it 'builds tenant route arguments and options with the resolved household slug' do
      Current.household = nil
      household = household_for('Route Arguments Household')
      record = Location.create!(name: 'Route Arguments Location', household: household)
      query = helper_query(user: user)

      expect(query.tenant_route_args(record)).to eq([household.slug, record])
      expect(query.tenant_route_options(record, anchor: 'result')).to eq(
        anchor: 'result',
        household_slug: household.slug
      )
    end

    it 'returns nil for records and users without household context' do
      Current.household = nil
      search_user = Struct.new(:person).new(nil)
      query = helper_query(user: search_user)

      expect(query.record_household(Object.new)).to be_nil
      expect(query.user_household_slug).to be_nil
      expect(query.household_slug_for(Object.new)).to be_nil
    end
  end
end
