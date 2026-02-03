# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Shared::PageHeader, type: :component do
  describe '#view_template' do
    it 'renders the title' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component).to_html

      expect(html).to include('Test Title')
    end

    it 'renders the subtitle when provided' do
      component = described_class.new(title: 'Test Title', subtitle: 'Test Subtitle')
      html = render_inline(component).to_html

      expect(html).to include('Test Subtitle')
    end

    it 'does not render subtitle when not provided' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component).to_html

      expect(html).not_to include('text-muted-foreground')
    end

    it 'renders mobile layout with md:hidden class' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component).to_html

      expect(html).to include('md:hidden')
    end

    it 'renders desktop layout with hidden md:block class' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component).to_html

      expect(html).to include('hidden md:block')
    end

    it 'renders actions when block is provided' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component) do |section|
        '<button>Action Button</button>'.html_safe if section == :actions
      end.to_html

      expect(html).to include('Action Button')
    end

    it 'has sticky positioning for mobile actions' do
      component = described_class.new(title: 'Test Title')
      html = render_inline(component) do |section|
        '<button>Action</button>'.html_safe if section == :actions
      end.to_html

      expect(html).to include('sticky top-16')
    end
  end
end
