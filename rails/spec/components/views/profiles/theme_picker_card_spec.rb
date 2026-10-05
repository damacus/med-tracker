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

  it 'renders palette options in a responsive grid' do
    rendered = render_inline(described_class.new)

    palette_grid = rendered.at_css('div.grid.grid-cols-2.gap-3.sm\\:grid-cols-3.xl\\:grid-cols-4')

    expect(palette_grid).to be_present
    expect(palette_grid.css('button[data-theme]').count).to eq(described_class::THEMES.count)
  end

  it 'uses token-driven surfaces instead of bespoke gradients and literal white fills' do
    rendered = render_inline(described_class.new)
    html = rendered.to_html

    banned_classes = ['bg-[radial-gradient', 'bg-white/80', "[font-family:'Outfit',sans-serif]",
                      'rounded-[2rem]', 'rounded-[1.5rem]', 'rounded-[1.1rem]', 'bg-card/95',
                      'bg-muted/45']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end
