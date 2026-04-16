# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::Users::SearchForm, type: :component do
  describe 'rendering without errors' do
    it 'renders the search form with default params' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('form')).to be_present
      expect(rendered.css('input[name="search"]')).to be_present
    end

    it 'renders role and status select elements using combobox components' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('[data-controller="ruby-ui--combobox"]')).to be_present
      expect(rendered.css('[data-ruby-ui--combobox-target="trigger"]')).to be_present
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

      expect(rendered.text).to include('All Roles')
      User.roles.each_key do |role|
        expect(rendered.text).to include(role.titleize)
      end
    end

    it 'pre-selects the current role filter' do
      rendered = render_inline(described_class.new(search_params: { role: 'administrator' }))

      expect(rendered.css('input[name="role"][value="administrator"][checked]')).to be_present
    end
  end

  describe 'status filter' do
    it 'renders active, inactive, and soft deleted status options' do
      rendered = render_inline(described_class.new)

      expect(rendered.text).to include('All')
      expect(rendered.text).to include('Active')
      expect(rendered.text).to include('Inactive')
      expect(rendered.text).to include('Soft deleted')
    end

    it 'pre-selects the current status filter' do
      rendered = render_inline(described_class.new(search_params: { status: 'active' }))

      expect(rendered.css('input[name="status"][value="active"][checked]')).to be_present
    end

    it 'pre-selects the soft deleted status filter' do
      rendered = render_inline(described_class.new(search_params: { status: 'soft_deleted' }))

      expect(rendered.css('input[name="status"][value="soft_deleted"][checked]')).to be_present
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
      expect(rendered.css('label[for="role_trigger"]')).to be_present
      expect(rendered.css('label[for="status_trigger"]')).to be_present
    end

    it 'associates labels with their inputs via matching ids' do
      rendered = render_inline(described_class.new)

      expect(rendered.css('input#search')).to be_present
      expect(rendered.css('button#role_trigger')).to be_present
      expect(rendered.css('button#status_trigger')).to be_present
    end
  end
end
