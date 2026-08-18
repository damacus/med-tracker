# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::Flash, type: :component do
  it 'renders messages for the layout notification region' do
    rendered = render_inline(described_class.new(notice: 'Saved'))

    expect(rendered.css('.container').text).to include('Saved')
  end

  describe 'notice flash' do
    it 'renders the notice message directly without redundant title' do
      rendered = render_inline(described_class.new(notice: 'Medication added successfully'))

      expect(rendered.text).to include('Medication added successfully')
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

    it 'lets the user dismiss the warning' do
      rendered = render_inline(described_class.new(warning: 'Set up 2FA'))

      button = rendered.at_css('button[aria-label="Close"]')
      expect(button['data-action']).to eq('click->flash#dismiss')
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
