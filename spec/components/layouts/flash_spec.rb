# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Flash, type: :component do
  it 'renders flash container above navigation' do
    rendered = render_inline(described_class.new(notice: 'Saved'))

    expect(rendered.to_html).to include('fixed')
    expect(rendered.to_html).to include('top-4')
    expect(rendered.to_html).to include('z-[60]')
  end

  describe 'notice flash' do
    it 'renders the notice message directly without redundant title' do
      rendered = render_inline(described_class.new(notice: 'Medicine added successfully'))

      expect(rendered.text).to include('Medicine added successfully')
      expect(rendered.text).not_to include('Success')
    end

    it 'renders the success icon' do
      rendered = render_inline(described_class.new(notice: 'Saved'))

      expect(rendered.css('svg').any?).to be true
    end
  end

  describe 'warning flash' do
    it 'renders the warning message with warning variant (amber styling)' do
      rendered = render_inline(described_class.new(warning: 'Please set up 2FA'))

      alert = rendered.css('[role="alert"]').first
      expect(rendered.text).to include('Please set up 2FA')
      expect(alert['class']).to include('warning'),
                                'Expected warning flash to use warning styling, not green (success)'
    end

    it 'renders the warning icon' do
      rendered = render_inline(described_class.new(warning: 'Set up 2FA'))

      expect(rendered.css('svg').any?).to be true
    end
  end

  describe 'alert flash' do
    it 'renders the alert message directly without redundant title' do
      rendered = render_inline(described_class.new(alert: 'Something went wrong'))

      expect(rendered.text).to include('Something went wrong')
      expect(rendered.text).not_to include('Error')
    end

    it 'renders the alert icon' do
      rendered = render_inline(described_class.new(alert: 'Failed'))

      expect(rendered.css('svg').any?).to be true
    end
  end
end
