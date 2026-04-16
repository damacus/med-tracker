# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::M3::Badge, type: :component do
  it 'renders filled badges with M3 classes instead of RubyUI container colors' do
    rendered = render_inline(described_class.new(variant: :filled)) { 'Badge' }

    badge = rendered.css('span').first
    expect(badge['class']).to include('bg-primary')
    expect(badge['class']).to include('text-on-primary')
    expect(badge['class']).not_to include('bg-primary-container')
  end

  it 'renders outlined badges with the M3 outline styling' do
    rendered = render_inline(described_class.new(variant: :outlined)) { 'Badge' }

    badge = rendered.css('span').first
    expect(badge['class']).to include('border-outline')
    expect(badge['class']).to include('text-primary')
  end
end
