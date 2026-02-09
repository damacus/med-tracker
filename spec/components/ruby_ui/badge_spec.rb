# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::Badge, type: :component do
  describe 'WCAG 2.2 SC 2.5.8 minimum target size' do
    %i[sm md lg].each do |size|
      it "includes min-h-[24px] and min-w-[24px] for :#{size} size" do
        rendered = render_inline(described_class.new(size: size)) { 'Test' }

        badge = rendered.css('span').first
        expect(badge['class']).to include('min-h-[24px]')
        expect(badge['class']).to include('min-w-[24px]')
      end
    end
  end

  it 'renders with primary variant by default' do
    rendered = render_inline(described_class.new) { 'Badge' }

    badge = rendered.css('span').first
    expect(badge['class']).to include('text-primary')
  end
end
