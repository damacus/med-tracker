# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::ThemePickerCard, type: :component do
  it 'renders theme options as accessible toggle buttons' do
    rendered = render_inline(described_class.new)

    buttons = rendered.css('button[data-theme]')

    expect(buttons).not_to be_empty
    expect(buttons.map { |button| button['type'] }).to all(eq('button'))
    expect(buttons.map { |button| button['aria-pressed'] }).to all(eq('false'))
  end
end
