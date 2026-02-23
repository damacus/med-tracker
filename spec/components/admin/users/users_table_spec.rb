# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::UsersTable, type: :component do
  fixtures :accounts, :people, :users

  let(:user_list) { User.all }
  let(:current_user) { users(:admin) }

  describe 'rendering' do
    it 'renders a table' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      expect(rendered.css('table')).to be_present
    end

    it 'renders table headers' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      headers = rendered.css('th').map(&:text)
      expect(headers).to include('Name', 'Email', 'Role', 'Activation', 'Verification', 'Actions')
    end

    it 'renders a row for each user' do
      rendered = render_inline(described_class.new(users: user_list, current_user: current_user))

      rows = rendered.css('tbody tr')
      expect(rows.length).to eq(user_list.count)
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
end
