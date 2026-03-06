# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::ThemePickerCard, type: :component do
  it 'renders appearance options as accessible toggle buttons' do
    rendered = render_inline(described_class.new)

    appearance_buttons = rendered.css('button[data-appearance]')

    expect(appearance_buttons.pluck('data-appearance')).to contain_exactly('light', 'dark', 'system')
    expect(appearance_buttons.pluck('type')).to all(eq('button'))
    expect(appearance_buttons.pluck('aria-pressed')).to all(eq('false'))
  end

  it 'renders palette options as accessible toggle buttons' do
    rendered = render_inline(described_class.new)

    theme_buttons = rendered.css('button[data-theme]')

    expect(theme_buttons).not_to be_empty
    expect(theme_buttons.pluck('type')).to all(eq('button'))
    expect(theme_buttons.pluck('aria-pressed')).to all(eq('false'))
  end
end
