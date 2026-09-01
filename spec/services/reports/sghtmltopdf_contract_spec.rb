require 'rails_helper'
require 'sghtmltopdf'
require 'zlib'

RSpec.describe Sghtmltopdf do
  it 'renders all supported report locales with the bundled Noto Sans font' do
    font_path = Rails.root.join('vendor/fonts/NotoSans-Regular.ttf').to_s

    described_class.configure do |config|
      config.gothic_font = font_path
    end

    pdf = described_class.render('<p>English · Cymraeg ŵ · Español ñ · Gaeilge á · Português ç</p>')
    streams = decompressed_streams(pdf)

    expect(pdf).to start_with('%PDF-1.7')
    expect(pdf).to include('/FontFile2')
    expect(font_program(streams)).to include("N\x00o\x00t\x00o\x00 \x00S\x00a\x00n\x00s".b)
    expect(unicode_map(streams)).to include('<0136> <0175>', '<00B3> <00F1>', '<00A3> <00E1>', '<00A9> <00E7>')
    expect(Rails.root.join('vendor/fonts/OFL-1.1.txt').read).to start_with(
      'Copyright 2018 The Noto Project Authors (github.com/googlei18n/noto-fonts)'
    )
  end

  it 'keeps report PDF rendering free of Prawn dependencies' do
    dependencies = Bundler.load.dependencies.map(&:name)

    expect(dependencies).to include('sghtmltopdf')
    expect(dependencies).not_to include('prawn', 'prawn-table')
  end

  def decompressed_streams(pdf)
    pdf.scan(/stream\r?\n(.*?)\r?\nendstream/m).filter_map do |(stream)|
      Zlib::Inflate.inflate(stream)
    rescue Zlib::DataError
      nil
    end
  end

  def unicode_map(streams)
    streams.find { it.include?('beginbfchar') }
  end

  def font_program(streams)
    streams.find { it.start_with?("\x00\x01\x00\x00".b) }
  end
end
