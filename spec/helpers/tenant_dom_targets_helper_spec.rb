# frozen_string_literal: true

require 'rails_helper'

RSpec.describe TenantDomTargetsHelper do
  after { Current.reset }

  it 'keeps legacy DOM ids outside household context' do
    person = build(:person, id: 123)

    expect(helper.tenant_dom_id(person)).to eq('person_123')
    expect(helper.tenant_dom_target('people')).to eq('people')
  end

  it 'prefixes DOM ids with household identity inside household context' do
    household = Household.create!(name: 'DOM Household', slug: 'dom-household')
    person = build(:person, id: 123, household: household)
    Current.household = household

    expect(helper.tenant_dom_id(person)).to eq("household_#{household.id}_person_123")
    expect(helper.tenant_dom_target('people')).to eq("household_#{household.id}_people")
  end
end
