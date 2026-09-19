# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AmbiguousPersonAccessGrants::IndexView, type: :component do
  let(:pagy) do
    Class.new do
      attr_reader :count, :from, :to, :pages, :page, :previous, :next

      def initialize
        @count = 25
        @from = 1
        @to = 10
        @pages = 3
        @page = 1
        @previous = nil
        @next = 2
      end
    end.new
  end

  it 'renders accessible navigation links when more than one page exists' do
    rendered = render_inline(described_class.new(grants: [], pagy: pagy))

    expect(rendered.css('nav[aria-label="Pagination"]')).to be_present
    expect(rendered.css('a').pluck('href')).to include(a_string_including('page=2'))
    expect(rendered.text).to include('Previous', 'Next')
  end

  it 'renders a reason field and the three classification decisions for each grant' do
    rendered = render_inline(described_class.new(grants: queue_rows))

    form = rendered.css('form[method="post"]')
    expect(form).to be_present
    expect(form.css('input[name="classification[carer_relationship_id]"]')).to be_present
    expect(form.css('input[name="classification[reason]"][required]')).to be_present
    decisions = form.css('button[name="classification[decision]"]').pluck('value')
    expect(decisions).to contain_exactly('attach', 'manual', 'revoke')
  end

  def queue_rows
    household = create(:household)
    account = Account.create!(email: 'component-carer@example.com',
                              password_hash: BCrypt::Password.create('password'), status: :verified)
    carer = create(:person, household: household, account: account, name: 'Queue Carer')
    patient = create(:person, household: household, name: 'Queue Patient')
    membership = household.household_memberships.create!(account: account, person: carer,
                                                         role: :member, status: :active)
    CarerRelationship.create!(household: household, carer: carer, patient: patient,
                              relationship_type: :parent, active: true)
    household.person_access_grants.create!(household_membership: membership, person: patient,
                                           access_level: :manage, relationship_type: :parent)

    Admin::AmbiguousPersonAccessGrantsIndexQuery.new(
      scope: PersonAccessGrant.where(household: household)
    ).call
  end
end
