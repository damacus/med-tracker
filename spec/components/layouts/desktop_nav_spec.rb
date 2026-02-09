# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::DesktopNav, type: :component do
  it 'renders nav links with nav__link class' do
    rendered = render_inline(described_class.new)

    links = rendered.css('a.nav__link')
    expect(links.length).to eq(3)
    expect(links.map(&:text)).to contain_exactly('Medicines', 'People', 'Medicine Finder')
  end
end
