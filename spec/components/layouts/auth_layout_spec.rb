# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::AuthLayout, type: :component do
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
