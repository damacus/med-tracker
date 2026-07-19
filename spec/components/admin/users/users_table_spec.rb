# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::UsersTable, type: :component do
  fixtures :accounts, :people, :users

  let(:user_list) { User.all }
  let(:current_user) { users(:admin) }
  let(:target_user) { users(:jane) }
  let(:household) { current_user.person.household }

  before do
    Current.household = household
    household.household_memberships.find_or_initialize_by(account: current_user.person.account).tap do |membership|
      membership.person = current_user.person
      membership.role = :owner
      membership.status = :active
      membership.save!
    end
    household.household_memberships.find_or_initialize_by(account: target_user.person.account).tap do |membership|
      membership.person = target_user.person
      membership.role = :administrator
      membership.status = :active
      membership.save!
    end
  end

  after { Current.reset }

  describe 'rendering' do
    it 'renders a table' do
      rendered = render_inline(table_component)

      expect(rendered.css('table')).to be_present
    end

    it 'renders mobile cards and keeps the desktop table' do
      rendered = render_inline(table_component)

      expect(rendered.css('[data-testid="admin-users-mobile-list"]')).to be_present
      expect(rendered.css('[data-testid="admin-users-desktop-table"] table')).to be_present
      mobile_list = rendered.css('[data-testid="admin-users-mobile-list"]')
      expect(mobile_list.text).to include(target_user.email_address, 'Household role', 'System administrator')
    end

    it 'keeps canonical row selectors unique when card representations render' do
      rendered = render_inline(table_component)

      expect(rendered.css("[data-user-id='#{target_user.id}']").length).to eq(1)
      expect(rendered.css("[data-user-card-id='#{target_user.id}']")).to be_present
      expect(rendered.css("[data-user-card-id='#{target_user.id}'] a").pluck('href')).to include(
        "/households/#{household.slug}/admin/users/#{target_user.id}/edit"
      )
    end

    it 'renders table headers' do
      rendered = render_inline(table_component)

      headers = rendered.css('th').map(&:text)
      expect(headers).to include(
        'Name', 'Email address', 'Household role', 'System administrator', 'Activation', 'Verification', 'Actions'
      )
    end

    it 'renders a row for each user' do
      rendered = render_inline(table_component)

      rows = rendered.css('tbody tr')
      expect(rows.length).to eq(user_list.count)
    end

    it 'renders edit links with outline button styling' do
      rendered = render_inline(table_component)

      row = rendered.css("[data-user-id='#{target_user.id}']").first
      edit_link = row.css('a').find { |link| link.text.include?('Edit') }
      expect(edit_link['href']).to include("/households/#{household.slug}/admin/users/#{target_user.id}/edit")
      expect(edit_link['class']).to include('border')
    end

    it 'renders the household membership role instead of the legacy user role' do
      rendered = render_inline(table_component(users: [target_user]))

      expect(rendered.text).to include('Administrator')
      expect(User.column_names).not_to include('role')
    end

    it 'bulk loads household membership roles for both table layouts' do
      users = User.where(id: [current_user.id, target_user.id]).includes(person: :account).to_a

      expect(count_membership_role_queries do
        render_inline(table_component(users: users))
      end).to eq(1)
    end

    it 'does not query row associations for users loaded by the index query' do
      users = Admin::UsersIndexQuery.new(
        scope: User.where(id: [current_user.id, target_user.id]),
        filters: {},
        household: household
      ).call.to_a

      component = table_component(users: users)

      expect(count_row_association_queries { render_inline(component) }).to eq(0)
    end

    it 'does not execute SQL while rendering preloaded users' do
      users = Admin::UsersIndexQuery.new(
        scope: User.where(id: [current_user.id, target_user.id]),
        filters: {},
        household: household
      ).call.to_a

      component = table_component(users: users)

      expect(count_queries { render_inline(component) }).to eq(0)
    end
  end

  describe 'sortable headers' do
    it 'renders sortable links for Name, Email, and Household role' do
      rendered = render_inline(table_component)

      sortable_links = rendered.css('th a')
      link_texts = sortable_links.map(&:text).join
      expect(link_texts).to include('Name')
      expect(link_texts).to include('Email')
      expect(link_texts).to include('Household role')
    end

    it 'includes sort direction in header links' do
      rendered = render_inline(table_component(search_params: { sort: 'name', direction: 'asc' }))

      name_link = rendered.css('th a').find { |a| a.text.include?('Name') }
      expect(name_link['href']).to include('direction=desc')
    end

    it 'renders sort indicator for active sort column' do
      rendered = render_inline(table_component(search_params: { sort: 'name', direction: 'asc' }))

      expect(rendered.text).to include('↑')
    end

    it 'renders descending indicator when direction is desc' do
      rendered = render_inline(table_component(search_params: { sort: 'name', direction: 'desc' }))

      expect(rendered.text).to include('↓')
    end
  end

  describe 'with empty user list' do
    it 'renders the table with no body rows' do
      rendered = render_inline(table_component(users: []))

      expect(rendered.css('tbody tr').length).to eq(0)
    end
  end

  def table_component(users: user_list, search_params: {})
    access_summary = Admin::UserAccessSummaryQuery.new(users: users, household: household).call
    described_class.new(
      users: users,
      access_summary: access_summary,
      search_params: search_params,
      current_user: current_user
    )
  end

  def count_membership_role_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      sql = payload[:sql]
      count += 1 if sql.include?('FROM "household_memberships"')
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  def count_row_association_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      sql = payload[:sql]
      count += 1 if sql.match?(/FROM "(people|accounts|platform_admins)"/)
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end

  def count_queries(&)
    count = 0
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'

      count += 1
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    count
  end
end
