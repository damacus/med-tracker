# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::MobileMenu, type: :component do
  describe 'i18n translations' do
    it 'renders mobile menu with Irish translations' do
      I18n.locale = :ga
      component = described_class.new

      rendered = render_inline(component)

      expect(rendered.to_html).to include('Dún roghchlár')
      expect(rendered.to_html).to include('Daoine')
      expect(rendered.to_html).to include('Aimsitheoir Leigheas')
      expect(rendered.to_html).to include('Próifíl')
      expect(rendered.to_html).to include('Logáil amach')
    end
  end
end
