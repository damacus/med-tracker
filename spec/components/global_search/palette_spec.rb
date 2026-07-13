# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::GlobalSearch::Palette, type: :component do
  it 'hides the close icon from the labelled close button' do
    rendered = render_inline(described_class.new)
    close_button = rendered.at_css('button[aria-label="Close search"]')

    expect(close_button).to be_present
    expect(close_button.at_css('svg[aria-hidden="true"]')).to be_present
  end
end
