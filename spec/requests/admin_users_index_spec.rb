# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin users index' do
  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }
  let(:soft_deleted_user) do
    account = Account.create!(
      email: 'soft.deleted@example.com',
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :closed
    )
    household = Household.find_by!(slug: default_url_options.fetch(:household_slug))
    person = Person.create!(
      name: 'Soft Deleted User',
      date_of_birth: '1990-01-01',
      household: household,
      account: account
    )
    household.household_memberships.create!(account: account, person: person, role: :member, status: :active)
    User.create!(
      person: person,
      email_address: 'soft.deleted@example.com',
      active: true
    )
  end

  before do
    sign_in(admin)
    soft_deleted_user
  end

  it 'shows soft deleted users to administrators' do
    get admin_users_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(soft_deleted_user.email_address)
    expect(response.body).to include('Soft deleted')
  end

  it 'explains household and system-administrator access separately' do
    PlatformAdmin.create!(account: users(:jane).person.account)

    get admin_users_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(
      'Household roles apply only to this household. System administrators can manage every household.'
    )
    expect(response.body).to include('Household role')
    expect(response.body).to include('System administrator')

    document = Nokogiri::HTML5(response.body)
    platform_admin_row = document.at_css("tr[data-user-id='#{users(:jane).id}']")
    ordinary_user_row = document.at_css("tr[data-user-id='#{admin.id}']")

    expect(platform_admin_row.css('td')[3].text.strip).to eq('Yes')
    expect(ordinary_user_row.css('td')[3].text.strip).to eq('No')
  end

  it 'filters the list to soft deleted users' do
    get admin_users_path, params: { status: 'soft_deleted' }

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(soft_deleted_user.email_address)
    expect(response.body).not_to include(users(:jane).email_address)
  end

  it 'filters the list to inactive users' do
    users(:jane).deactivate!

    get admin_users_path, params: { status: 'inactive' }

    expect(response).to have_http_status(:ok)
    expect(response.body).to include(users(:jane).email_address)
    expect(response.body).not_to include(admin.email_address)
  end
end
