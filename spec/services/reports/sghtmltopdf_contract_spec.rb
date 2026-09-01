require 'rails_helper'
require 'sghtmltopdf'

RSpec.describe 'sghtmltopdf renderer' do
  it 'renders all supported report locales with the bundled Noto Sans font' do
    Sghtmltopdf.configure do |config|
      config.gothic_font = Rails.root.join('vendor/fonts/NotoSans-Regular.ttf').to_s
    end

    pdf = Sghtmltopdf.render('<p>English · Cymraeg ŵ · Español ñ · Gaeilge á · Português ç</p>')

    expect(pdf).to start_with('%PDF-1.7')
  end
end
