# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Alert, type: :component do
  it 'renders with role alert' do
    rendered = render_inline(described_class.new)

    expect(rendered.css("div[role='alert']")).to be_present
  end

  it 'merges custom classes' do
    rendered = render_inline(described_class.new(class: 'custom-class'))

    expect(rendered.css('.custom-class')).to be_present
  end
end
