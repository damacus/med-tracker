# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::SearchForm, type: :component do
  describe 'rendering without errors' do
    it 'renders the search form with default params' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('form')).to be_present
      expect(rendered.css('input[name="search"]')).to be_present
    end

    it 'renders role and status select elements using select_classes from FormHelpers' do
      rendered = render_inline(described_class.new)

      role_select = rendered.css('select[name="role"]').first
      status_select = rendered.css('select[name="status"]').first

      expect(role_select).to be_present
      expect(status_select).to be_present
      expect(role_select['class']).to include('rounded-md')
      expect(status_select['class']).to include('rounded-md')
    end
  end

  describe 'search field' do
    it 'renders a text input for search' do
      rendered = render_inline(described_class.new)

      search_input = rendered.css('input[name="search"]').first
      expect(search_input['type']).to eq('text')
      expect(search_input['placeholder']).to include('Search')
    end

    it 'populates the search field with existing params' do
      rendered = render_inline(described_class.new(search_params: { search: 'john' }))

      search_input = rendered.css('input[name="search"]').first
      expect(search_input['value']).to eq('john')
    end
  end

  describe 'role filter' do
    it 'renders all user roles as options' do
      rendered = render_inline(described_class.new)

      role_select = rendered.css('select[name="role"]').first
      options = role_select.css('option')

      expect(options.first.text).to eq('All Roles')
      User.roles.each_key do |role|
        expect(options.map(&:text)).to include(role.titleize)
      end
    end

    it 'pre-selects the current role filter' do
      rendered = render_inline(described_class.new(search_params: { role: 'administrator' }))

      role_select = rendered.css('select[name="role"]').first
      selected = role_select.css('option[selected]')
      expect(selected.first['value']).to eq('administrator')
    end
  end

  describe 'status filter' do
    it 'renders active and inactive status options' do
      rendered = render_inline(described_class.new)

      status_select = rendered.css('select[name="status"]').first
      options = status_select.css('option').map(&:text)

      expect(options).to include('All', 'Active', 'Inactive')
    end

    it 'pre-selects the current status filter' do
      rendered = render_inline(described_class.new(search_params: { status: 'active' }))

      status_select = rendered.css('select[name="status"]').first
      selected = status_select.css('option[selected]')
      expect(selected.first['value']).to eq('active')
    end
  end

  describe 'actions' do
    it 'renders a search button' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('button[type="submit"]')).to be_present
      expect(rendered.text).to include('Search')
    end

    it 'renders a clear link when filters are active' do
      rendered = render_inline(described_class.new(search_params: { search: 'test' }))

      expect(rendered.text).to include('Clear')
    end

    it 'does not render a clear link when no filters are active' do
      rendered = render_inline(described_class.new(search_params: {}))

      expect(rendered.text).not_to include('Clear')
    end
  end

  describe 'accessibility' do
    it 'renders labels for all form fields' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('label[for="search"]')).to be_present
      expect(rendered.css('label[for="role"]')).to be_present
      expect(rendered.css('label[for="status"]')).to be_present
    end

    it 'associates labels with their inputs via matching ids' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('input#search')).to be_present
      expect(rendered.css('select#role')).to be_present
      expect(rendered.css('select#status')).to be_present
    end
  end
end
