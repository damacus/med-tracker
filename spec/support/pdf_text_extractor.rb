# frozen_string_literal: true

require 'zlib'

module PdfTextExtractor
  def pdf_metadata(pdf)
    pdf.scan(%r{/(Title|Author|Subject|Keywords) \((.*?)\)}).to_h
  end

  def pdf_text(pdf)
    streams = pdf_streams(pdf)
    text = pdf_character_maps(streams).product(pdf_text_streams(streams)).flat_map do |character_map, stream|
      decoded_pdf_text(character_map, stream)
    end.join(' ')
    "#{text} #{pdf_link_text(pdf)}".unicode_normalize(:nfkd).gsub(/\s+/, ' ').strip
  end

  def pdf_streams(pdf)
    pdf.scan(/stream\r?\n(.*?)\r?\nendstream/m).filter_map do |(stream)|
      Zlib::Inflate.inflate(stream)
    rescue Zlib::Error
      nil
    end
  end

  def pdf_character_maps(streams)
    streams.filter_map { |stream| character_map(stream).presence }
  end

  def pdf_text_streams(streams)
    streams.grep(/(?:\)|>)\s*(?:Tj|['"])|\] TJ/)
  end

  def decoded_pdf_text(character_map, stream)
    pdf_text_operators(stream).filter_map do |operator|
      decoded = decoded_pdf_string(operator)
      next unless decoded.bytesize.even?

      decoded.unpack('n*').filter_map { |cid| character_map[cid] }.join
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

  def pdf_text_operators(stream)
    direct_text_operators(stream) + array_text_operators(stream)
  end

  def direct_text_operators(stream)
    stream.scan(/(\((?:\\.|[^\\)])*\)|<[0-9A-F]+>)\s*(?:Tj|['"])/im).flatten
  end

  def array_text_operators(stream)
    stream.scan(/\[(.*?)\]\s*TJ/m).flat_map do |(array)|
      array.scan(/\((?:\\.|[^\\)])*\)|<[0-9A-F]+>/i)
    end
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
    character_maps = pdf_character_maps(pdf_streams(pdf))

    pdf_page_content_streams(pdf).map do |streams|
      character_maps.product(streams).flat_map do |character_map, stream|
        decoded_pdf_text(character_map, stream)
      end.join(' ').unicode_normalize(:nfkd).gsub(/\s+/, ' ').strip
    end
  end

  def pdf_page_content_streams(pdf)
    objects = pdf_objects(pdf)

    pdf_page_references(objects).map do |reference|
      pdf_page_content_references(objects.fetch(reference)).filter_map do |content_reference|
        pdf_object_stream(objects.fetch(content_reference))
      end
    end
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
    stream = object[/stream\r?\n(.*?)\r?\nendstream/m, 1]
    Zlib::Inflate.inflate(stream)
  rescue Zlib::Error, TypeError
    nil
  end
end

RSpec.configure do |config|
  config.include PdfTextExtractor
  config.include PdfPageTextExtractor
end
