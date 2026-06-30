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
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      expect(rendered.css('table')).to be_present
    end

    it 'renders mobile cards and keeps the desktop table' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      expect(rendered.css('[data-testid="admin-users-mobile-list"]')).to be_present
      expect(rendered.css('[data-testid="admin-users-desktop-table"] table')).to be_present
      expect(rendered.css('[data-testid="admin-users-mobile-list"]').text).to include(target_user.email_address)
    end

    it 'keeps canonical row selectors unique when card representations render' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      expect(rendered.css("[data-user-id='#{target_user.id}']").length).to eq(1)
      expect(rendered.css("[data-user-card-id='#{target_user.id}']")).to be_present
      expect(rendered.css("[data-user-card-id='#{target_user.id}'] a").pluck('href')).to include(
        "/households/#{household.slug}/admin/users/#{target_user.id}/edit"
      )
    end

    it 'renders table headers' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      headers = rendered.css('th').map(&:text)
      expect(headers).to include('Name', 'Email address', 'Role', 'Activation', 'Verification', 'Actions')
    end

    it 'renders a row for each user' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      rows = rendered.css('tbody tr')
      expect(rows.length).to eq(user_list.count)
    end

    it 'renders edit links with outline button styling' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      row = rendered.css("[data-user-id='#{target_user.id}']").first
      edit_link = row.css('a').find { |link| link.text.include?('Edit') }
      expect(edit_link['href']).to include("/households/#{household.slug}/admin/users/#{target_user.id}/edit")
      expect(edit_link['class']).to include('border')
    end

    it 'renders the household membership role instead of the legacy user role' do
      rendered = render_inline(described_class.new(users: [target_user], current_user: current_user))

      expect(rendered.text).to include('Administrator')
      expect(User.column_names).not_to include('role')
    end

    it 'bulk loads household membership roles for both table layouts' do
      users = User.where(id: [current_user.id, target_user.id]).includes(person: :account).to_a

      expect(count_membership_role_queries do
        render_inline(described_class.new(users: users, current_user: current_user, household: household))
      end).to eq(1)
    end
  end

  describe 'sortable headers' do
    it 'renders sortable links for Name, Email, and Role' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      sortable_links = rendered.css('th a')
      link_texts = sortable_links.map(&:text).join
      expect(link_texts).to include('Name')
      expect(link_texts).to include('Email')
      expect(link_texts).to include('Role')
    end

    it 'includes sort direction in header links' do
      rendered = render_inline(described_class.new(
                                 users: user_list,
                                 search_params: { sort: 'name', direction: 'asc' },
                                 current_user: current_user
                               ))

      name_link = rendered.css('th a').find { |a| a.text.include?('Name') }
      expect(name_link['href']).to include('direction=desc')
    end

    it 'renders sort indicator for active sort column' do
      rendered = render_inline(described_class.new(
                                 users: user_list,
                                 search_params: { sort: 'name', direction: 'asc' },
                                 current_user: current_user
                               ))

      expect(rendered.text).to include('↑')
    end

    it 'renders descending indicator when direction is desc' do
      rendered = render_inline(described_class.new(
                                 users: user_list,
                                 search_params: { sort: 'name', direction: 'desc' },
                                 current_user: current_user
                               ))

      expect(rendered.text).to include('↓')
    end
  end

  describe 'with empty user list' do
    it 'renders the table with no body rows' do
      rendered = render_inline(described_class.new(users: [], current_user: current_user))

      expect(rendered.css('tbody tr').length).to eq(0)
    end
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
end
