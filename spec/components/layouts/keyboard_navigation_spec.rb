# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Keyboard navigation', type: :component do # rubocop:disable RSpec/DescribeClass
  fixtures :accounts, :people, :users

  describe Components::Layouts::Navigation do
    before do
      allow_any_instance_of(described_class).to receive(:authenticated?).and_return(false) # rubocop:disable RSpec/AnyInstance
    end

    it 'renders a skip-to-content link as the first focusable element' do
      rendered = render_inline(described_class.new)

      skip_link = rendered.css('a[href="#main-content"]').first
      expect(skip_link).to be_present
      expect(skip_link.text.strip).to eq('Skip to content')
    end

    it 'renders the skip link with sr-only and focus-visible styles' do
      rendered = render_inline(described_class.new)

      skip_link = rendered.css('a[href="#main-content"]').first
      expect(skip_link['class']).to include('sr-only')
      expect(skip_link['class']).to include('focus:not-sr-only')
    end
  end

  describe Components::Layouts::DesktopNav do
    it 'renders nav links with focus-visible ring styles' do
      rendered = render_inline(described_class.new)

      links = rendered.css('a.nav__link')
      expect(links).to be_present
    end
  end

  describe Components::Layouts::BottomNav do
    it 'renders nav links with focus-visible styles' do
      rendered = render_inline(described_class.new)

      links = rendered.css('a')
      links.each do |link|
        classes = link['class'] || ''
        expect(classes).to include('focus-visible:ring-2'),
                           "Bottom nav link '#{link.text.strip}' missing focus-visible:ring-2"
      end
    end
  end

  describe Components::Admin::Users::SearchForm do
    it 'renders select elements with focus ring styles' do
      rendered = render_inline(described_class.new)

      selects = rendered.css('select')
      selects.each do |select_el|
        classes = select_el['class'] || ''
        expect(classes).to include('focus:ring'),
                           "Select '#{select_el['name']}' missing focus:ring style for keyboard visibility"
      end
    end
  end
end
