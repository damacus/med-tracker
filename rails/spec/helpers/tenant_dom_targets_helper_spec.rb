# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TenantDomTargetsHelper do
  after { Current.reset }

  it 'keeps legacy DOM ids outside household context' do
    Current.household = nil
    person = build(:person, id: 123)

    expect(helper.tenant_dom_id(person)).to eq('person_123')
    expect(helper.tenant_dom_target('people')).to eq('people')
  end

  it 'prefixes DOM ids with household identity inside household context' do
    household = Household.create!(name: 'DOM Household', slug: 'dom-household')
    other_household = Household.create!(name: 'Other DOM Household', slug: 'other-dom-household')
    person = build(:person, id: 123, household: household)
    Current.household = household

    household_target = helper.tenant_dom_id(person)
    Current.household = other_household
    other_household_target = helper.tenant_dom_id(person)

    expect(household_target).to eq("household_#{household.id}_person_123")
    expect(other_household_target).to eq("household_#{other_household.id}_person_123")
    expect(other_household_target).not_to eq(household_target)

    Current.household = household
    expect(helper.tenant_dom_target('people')).to eq("household_#{household.id}_people")
  end
end
