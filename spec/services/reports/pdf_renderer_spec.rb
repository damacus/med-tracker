require 'rails_helper'
require 'zlib'

class PdfRendererTableContent < Phlex::HTML
  def initialize(row_count)
    @row_count = row_count
    super()
  end

  def view_template
    table do
      render_table_header
      render_table_body
    end
  end

  private

  def render_table_header
    thead do
      tr do
        th { 'Name' }
        th { 'Detail' }
      end
    end
  end

  def render_table_body
    tbody do
      @row_count.times do |index|
        tr do
          td { "Item #{index}" }
          td { 'A detailed report row.' }
        end
      end
    end
  end
end

RSpec.describe Reports::PdfRenderer do
  let(:renderer) { described_class.new }
  let(:generated_at) { Time.utc(2026, 9, 1, 10, 30) }
  let(:metadata) do
    {
      title: 'MedTracker shared report',
      author: 'MedTracker',
      subject: 'A print-safe report',
      keywords: 'medication, report'
    }
  end

  it 'renders complete report components with PDF metadata and readable text', :aggregate_failures do
    pdf = renderer.render(component: document_with('A prepared report body.'), metadata:)
    streams = decompressed_streams(pdf)

    expect(pdf).to start_with('%PDF-1.7')
    expect(pdf).to include(
      '/Title (MedTracker shared report)',
      '/Author (MedTracker)',
      '/Subject (A print-safe report)',
      '/Keywords (medication, report)'
    )
    expect(extracted_text(streams)).to include('Report section')
  end

  it 'embeds the explicit bundled font for every supported report locale', :aggregate_failures do
    pdf = renderer.render(
      component: document_with('English · Cymraeg ŵ · Español ñ · Gaeilge á · Português ç'),
      metadata:
    )
    streams = decompressed_streams(pdf)

    expect(pdf).to include('/FontFile2')
    expect(font_program(streams)).to include("N\x00o\x00t\x00o\x00 \x00S\x00a\x00n\x00s".b)
    expect(unicode_map(streams)).to include('<0136> <0175>', '<00B3> <00F1>', '<00A3> <00E1>', '<00A9> <00E7>')
  end

  it 'renders empty, short, long and multi-page report content synchronously', :aggregate_failures do
    empty_pdf = renderer.render(component: document_with('No report items are available.', empty: true), metadata:)
    short_pdf = renderer.render(component: document_with('Short report content.'), metadata:)
    long_pdf = renderer.render(component: document_with('Long report content. ' * 800), metadata:)
    multi_page_pdf = renderer.render(component: document_with_table(240), metadata:)

    expect(empty_pdf).to start_with('%PDF-1.7')
    expect(short_pdf).to start_with('%PDF-1.7')
    expect(long_pdf).to start_with('%PDF-1.7')
    expect(multi_page_pdf.scan(%r{/Type /Page\b}).size).to be > 1
  end

  it 'rejects remote and arbitrary local assets before rendering' do
    remote_component = document_with_image('https://example.test/report.png')
    local_component = document_with_image('file:///etc/passwd')
    bundled_font_component = document_with_image("file://#{Rails.root.join('vendor/fonts/NotoSans-Regular.ttf')}")

    allow(Sghtmltopdf).to receive(:render).and_return('%PDF-1.7')

    expect { renderer.render(component: remote_component, metadata:) }.to raise_error(described_class::Error)
    expect { renderer.render(component: local_component, metadata:) }.to raise_error(described_class::Error)
    expect(renderer.render(component: bundled_font_component, metadata:)).to eq('%PDF-1.7')
  end

  it 'rejects inline CSS, CSS imports, srcset, SVG and related resource attributes before rendering' do
    blocked_components = {
      inline_style: document_with_inline_style('background-image: url(https://example.test/report.png)'),
      stylesheet_import: document_with_stylesheet('@import "https://example.test/report.css";'),
      srcset: document_with_srcset('https://example.test/report.png 1x'),
      svg_href: document_with_svg_href('https://example.test/report.svg'),
      svg_xlink_href: document_with_svg_xlink_href('https://example.test/report.svg'),
      video_poster: document_with_video_poster('https://example.test/poster.png'),
      script_source: document_with_script_source('https://example.test/report.js')
    }
    allow(Sghtmltopdf).to receive(:render).and_return('%PDF-1.7')

    blocked_components.each do |name, component|
      expect { renderer.render(component:, metadata:) }.to raise_error(described_class::Error), name.to_s
    end

    expect(Sghtmltopdf).not_to have_received(:render)
  end

  it 'rejects CSS imports with no whitespace or intervening comments before rendering' do
    blocked_components = {
      no_whitespace: document_with_stylesheet('@import"https://example.test/report.css";'),
      comment_whitespace: document_with_stylesheet('@import/**/"file:///etc/passwd";')
    }
    allow(Sghtmltopdf).to receive(:render).and_return('%PDF-1.7')

    blocked_components.each do |name, component|
      expect { renderer.render(component:, metadata:) }.to raise_error(described_class::Error), name.to_s
    end

    expect(Sghtmltopdf).not_to have_received(:render)
  end

  it 'normalizes only known renderer errors and preserves their causes' do
    renderer_error = Sghtmltopdf::Error.new('engine failed')
    allow(Sghtmltopdf).to receive(:render).and_raise(renderer_error)

    expect { renderer.render(component: document_with('A report body.'), metadata:) }
      .to raise_error(described_class::Error) { |error| expect(error.cause).to equal(renderer_error) }
  end

  it 'does not normalize unexpected errors' do
    allow(Sghtmltopdf).to receive(:render).and_raise(ArgumentError, 'unexpected failure')

    expect { renderer.render(component: document_with('A report body.'), metadata:) }
      .to raise_error(ArgumentError, 'unexpected failure')
  end

  def document_with(body, empty: false)
    Components::Reports::PdfDocument.new(
      title: 'Shared report',
      context: 'All people',
      generated_at:,
      content: sample_content(body, empty:)
    )
  end

  def document_with_table(row_count)
    Components::Reports::PdfDocument.new(
      title: 'Long report',
      context: 'All people',
      generated_at:,
      content: PdfRendererTableContent.new(row_count)
    )
  end

  def document_with_image(source)
    Components::Reports::PdfDocument.new(
      title: 'Unsafe report',
      context: 'All people',
      generated_at:,
      content: Class.new(Phlex::HTML) { define_method(:view_template) { img(src: source) } }.new
    )
  end

  def document_with_inline_style(style)
    document_with_content(Class.new(Phlex::HTML) { define_method(:view_template) { div(style:) } }.new)
  end

  def document_with_stylesheet(stylesheet)
    document_with_content(Class.new(Phlex::HTML) do
      define_method(:view_template) { style { raw safe(stylesheet) } }
    end.new)
  end

  def document_with_srcset(srcset)
    document_with_content(Class.new(Phlex::HTML) { define_method(:view_template) { img(srcset:) } }.new)
  end

  def document_with_svg_href(href)
    document_with_content(Class.new(Phlex::HTML) do
      define_method(:view_template) { raw safe("<svg><image href=\"#{href}\" /></svg>") }
    end.new)
  end

  def document_with_svg_xlink_href(href)
    document_with_content(Class.new(Phlex::HTML) do
      define_method(:view_template) { raw safe("<svg><image xlink:href=\"#{href}\" /></svg>") }
    end.new)
  end

  def document_with_video_poster(poster)
    document_with_content(Class.new(Phlex::HTML) { define_method(:view_template) { video(poster:) } }.new)
  end

  def document_with_script_source(source)
    document_with_content(Class.new(Phlex::HTML) { define_method(:view_template) { script(src: source) } }.new)
  end

  def document_with_content(content)
    Components::Reports::PdfDocument.new(
      title: 'Unsafe report',
      context: 'All people',
      generated_at:,
      content:
    )
  end

  def sample_content(body, empty:)
    Class.new(Phlex::HTML) do
      define_method(:view_template) do
        section(class: empty ? 'empty-state' : 'report-section') do
          h2 { 'Report section' }
          p { body }
        end
      end
    end.new
  end

  def decompressed_streams(pdf)
    pdf.scan(/stream\r?\n(.*?)\r?\nendstream/m).filter_map do |(stream)|
      Zlib::Inflate.inflate(stream)
    rescue Zlib::Error
      nil
    end
  end

  def unicode_map(streams)
    streams.find { it.include?('beginbfchar') }
  end

  def font_program(streams)
    streams.find { it.start_with?("\x00\x01\x00\x00".b) }
  end

  def extracted_text(streams)
    pdf_character_maps(streams).product(pdf_text_streams(streams)).flat_map do |character_map, stream|
      decoded_pdf_text(character_map, stream)
    end.join
  end

  def pdf_character_maps(streams)
    streams.grep(/beginbfchar/).map { |stream| character_map(stream) }
  end

  def pdf_text_streams(streams)
    streams.grep(/\) Tj/)
  end

  def decoded_pdf_text(character_map, stream)
    stream.scan(/\((.*?)\) Tj/m).flatten.filter_map do |string|
      decoded = string.gsub(/\\([0-7]{3})/) { Regexp.last_match(1).to_i(8).chr }
      next unless decoded.bytesize.even?

      decoded.unpack('n*').filter_map { |cid| character_map[cid] }.join
    end
  end

  def character_map(stream)
    stream.scan(/<([0-9A-F]{4})> <([0-9A-F]{4})>/).to_h do |cid, codepoint|
      [cid.to_i(16), codepoint.to_i(16).chr(Encoding::UTF_8)]
    end
  end
end
