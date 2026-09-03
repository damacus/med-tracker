# frozen_string_literal: true

require 'zlib'

module PdfTextExtractor
  def pdf_metadata(pdf)
    pdf.scan(%r{/(Title|Author|Subject|Keywords) \((.*?)\)}).to_h
  end

  def pdf_text(pdf)
    text = pdf_page_texts(pdf).join(' ')
    "#{text} #{pdf_link_text(pdf)}".unicode_normalize(:nfkd).gsub(/\s+/, ' ').strip
  end

  def pdf_streams(pdf)
    pdf.scan(/stream\r?\n(.*?)\r?\nendstream/m).filter_map do |(stream)|
      Zlib::Inflate.inflate(stream)
    rescue Zlib::Error
      nil
    end
  end

  def decoded_pdf_string(operator)
    string = operator[1...-1]
    return [string].pack('H*') if operator.start_with?('<')

    string
      .gsub(/\\([0-7]{1,3})/) { Regexp.last_match(1).to_i(8).chr }
      .gsub(/\\([()\\])/) { Regexp.last_match(1) }
  end

  def character_map(stream)
    bfchar_map(stream).merge(bfrange_map(stream))
  end

  def bfchar_map(stream)
    characters = cmap_blocks(stream, 'bfchar').flat_map do |block|
      block.scan(/<([0-9A-F]{4})>\s+<([0-9A-F]{4})>/i)
    end
    characters.to_h do |cid, codepoint|
      [cid.to_i(16), codepoint.to_i(16).chr(Encoding::UTF_8)]
    end
  end

  def bfrange_map(stream)
    cmap_blocks(stream, 'bfrange').flat_map do |block|
      direct_bfrange_map(block) + array_bfrange_map(block)
    end.to_h
  end

  def direct_bfrange_map(block)
    block.scan(/<([0-9A-F]{4})>\s+<([0-9A-F]{4})>\s+<([0-9A-F]{4})>/i).flat_map do |first, last, codepoint|
      (first.to_i(16)..last.to_i(16)).map do |cid|
        [cid, (codepoint.to_i(16) + cid - first.to_i(16)).chr(Encoding::UTF_8)]
      end
    end
  end

  def array_bfrange_map(block)
    block.scan(/<([0-9A-F]{4})>\s+<([0-9A-F]{4})>\s+\[(.*?)\]/im).flat_map do |first, _last, codepoints|
      codepoints.scan(/<([0-9A-F]{4})>/i).flatten.each_with_index.map do |codepoint, index|
        [first.to_i(16) + index, codepoint.to_i(16).chr(Encoding::UTF_8)]
      end
    end
  end

  def cmap_blocks(stream, name)
    stream.scan(/begin#{name}(.*?)end#{name}/m).flatten
  end

  def pdf_link_text(pdf)
    pdf.scan(%r{/URI\s*\((.*?)\)}m).flatten.map { |uri| unescape_pdf_string(uri) }.join
  end

  def unescape_pdf_string(string)
    string.gsub(/\\([()\\])/) { Regexp.last_match(1) }
  end
end

module PdfPageTextExtractor
  def pdf_page_texts(pdf)
    objects = pdf_objects(pdf)

    pdf_page_references(objects).map do |reference|
      page = objects.fetch(reference)
      font_maps = pdf_page_font_maps(page, objects)
      streams = pdf_page_content_references(page).filter_map do |content_reference|
        pdf_object_stream(objects.fetch(content_reference))
      end

      streams
        .filter_map { |stream| decode_page_stream(stream, font_maps) }
        .join(' ')
        .unicode_normalize(:nfkd).gsub(/\s+/, ' ').strip
    end
  end

  def pdf_page_font_maps(page, objects)
    font_section = pdf_font_section(page, objects)

    font_section.scan(%r{/([A-Za-z0-9]+)\s+(\d+)\s+0\s+R}).each_with_object({}) do |(name, reference), maps|
      maps[name] = pdf_font_map(objects.fetch(reference), objects)
    end
  end

  def pdf_font_section(page, objects)
    resources_reference = page[%r{/Resources\s+(\d+)\s+0\s+R}, 1]
    resources = resources_reference ? objects.fetch(resources_reference) : page
    font_dictionary = resources[%r{/Font\s+(<<.*?>>|\d+\s+0\s+R)}m, 1].to_s
    reference = font_dictionary[/([0-9]+)\s+0\s+R/, 1]
    font_dictionary = objects.fetch(reference) if reference && !font_dictionary.start_with?('<<')
    font_dictionary[/<<(.*?)>>/m, 1].to_s
  end

  def pdf_font_map(font, objects)
    unicode_reference = font[%r{/ToUnicode\s+(\d+)\s+0\s+R}, 1]
    unicode_stream = pdf_object_stream(objects.fetch(unicode_reference)) if unicode_reference
    character_map(unicode_stream) if unicode_stream
  end

  def decode_page_stream(stream, font_maps)
    font_name = nil
    text = []

    stream.scan(pdf_page_operator_pattern) do |selected_font, direct, _array, array_body|
      font_name = selected_font if selected_font
      operators = direct ? [direct] : array_body.to_s.scan(/\((?:\\.|[^\\)])*\)|<[0-9A-F]+>/i)
      decoded = operators.filter_map { |operator| decode_operator(operator, font_maps[font_name]) }.join
      text << decoded unless decoded.empty?
    end

    text.join(' ')
  end

  def pdf_page_operator_pattern
    %r{/([A-Za-z0-9]+)\s+[-\d.]+\s+Tf|(\((?:\\.|[^\\)])*\)|<[0-9A-F]+>)\s*Tj|(\[(.*?)\])\s*TJ}im
  end

  def decode_operator(operator, character_map)
    return unless character_map

    decoded = decoded_pdf_string(operator)
    return unless decoded.bytesize.even?

    decoded.unpack('n*').filter_map { |cid| character_map[cid] }.join
  end

  def pdf_page_references(objects)
    catalog = objects.values.find { it.match?(%r{/Type /Catalog\b}) }
    pages_reference = catalog[%r{/Pages\s+(\d+)\s+\d+\s+R}, 1]

    pdf_page_tree_references(objects, pages_reference)
  end

  def pdf_page_tree_references(objects, reference)
    object = objects.fetch(reference)
    return [reference] if object.match?(%r{/Type /Page\b})

    pdf_page_kid_references(object).flat_map do |kid_reference|
      pdf_page_tree_references(objects, kid_reference)
    end
  end

  def pdf_page_kid_references(page_tree)
    page_tree[%r{/Kids\s*\[(.*?)\]}m, 1].scan(/(\d+)\s+\d+\s+R/).flatten
  end

  def pdf_objects(pdf)
    pdf.scan(/(\d+)\s+\d+\s+obj\s*(.*?)\s*endobj/m).to_h
  end

  def pdf_page_content_references(page_object)
    contents = page_object[%r{/Contents\s+(?:\[(.*?)\]|(\d+)\s+\d+\s+R)}m, 1] ||
               page_object[%r{/Contents\s+(?:\[(.*?)\]|(\d+)\s+\d+\s+R)}m, 2]
    return [] unless contents

    contents.scan(/(\d+)\s+\d+\s+R/).flatten.presence || [contents]
  end

  def pdf_object_stream(object)
    stream_header = object.match(/stream\r?\n/)
    return unless stream_header

    length = object[%r{/Length\s+(\d+)}, 1]&.to_i
    stream = object.byteslice(stream_header.end(0), length) || object.byteslice(stream_header.end(0)..)
    return unless stream

    Zlib::Inflate.inflate(stream)
  rescue Zlib::Error, TypeError
    stream
  end
end

RSpec.configure do |config|
  config.include PdfTextExtractor
  config.include PdfPageTextExtractor
end
