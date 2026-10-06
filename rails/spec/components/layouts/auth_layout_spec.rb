# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::AuthLayout, type: :component do
  it 'preloads the auth typeface before rendering the stylesheet' do
    rendered = Nokogiri::HTML5(described_class.new.render_in(controller.view_context))
    preloads = rendered.css('head link[rel="preload"][as="font"]')

    expect(preloads.pluck('href')).to contain_exactly(
      '/fonts/plus-jakarta-sans/plus-jakarta-sans-v12-latin-regular.woff2',
      '/fonts/plus-jakarta-sans/plus-jakarta-sans-v12-latin-500.woff2',
      '/fonts/plus-jakarta-sans/plus-jakarta-sans-v12-latin-600.woff2',
      '/fonts/plus-jakarta-sans/plus-jakarta-sans-v12-latin-700.woff2'
    )
    expect(preloads).to all(
      satisfy { |link| link['type'] == 'font/woff2' && link['crossorigin'] == 'anonymous' }
    )
  end

  it 'does not render global flash messages (auth views handle flash inline)' do
    allow(controller).to receive(:flash).and_return(
      ActionDispatch::Flash::FlashHash.new(alert: 'Please login to continue')
    )

    rendered = render_inline(described_class.new)

    flash_div = rendered.css('#flash').first
    expect(flash_div.children.length).to eq(0),
                                         'AuthLayout should not render global flash — auth views ' \
                                         'render flash inline near the content per proximity principle.'
  end
end
