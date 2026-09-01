# frozen_string_literal: true

require 'zlib'

module PdfTextExtractor
  def pdf_metadata(pdf)
    pdf.scan(%r{/(Title|Author|Subject|Keywords) \((.*?)\)}).to_h
  end

  def pdf_text(pdf)
    streams = pdf_streams(pdf)
    pdf_character_maps(streams).product(pdf_text_streams(streams)).flat_map do |character_map, stream|
      decoded_pdf_text(character_map, stream)
    end.join
  end

  def pdf_streams(pdf)
    pdf.scan(/stream\r?\n(.*?)\r?\nendstream/m).filter_map do |(stream)|
      Zlib::Inflate.inflate(stream)
    rescue Zlib::Error
      nil
    end
  end

  def pdf_character_maps(streams)
    streams.grep(/beginbfchar/).map do |stream|
      stream.scan(/<([0-9A-F]{4})> <([0-9A-F]{4})>/).to_h do |cid, codepoint|
        [cid.to_i(16), codepoint.to_i(16).chr(Encoding::UTF_8)]
      end
    end
  end

  def pdf_text_streams(streams)
    streams.grep(/(?:\)|>) Tj/)
  end

  def decoded_pdf_text(character_map, stream)
    stream.scan(/(?:\((?:\\.|[^\\)])*\)|<[0-9A-F]+>) Tj/m).filter_map do |operator|
      decoded = decoded_pdf_string(operator)
      next unless decoded.bytesize.even?

      decoded.unpack('n*').filter_map { |cid| character_map[cid] }.join
    end
  end

  def decoded_pdf_string(operator)
    string = operator[1...-4]
    return [string].pack('H*') if operator.start_with?('<')

    string.gsub(/\\([0-7]{1,3})/) { Regexp.last_match(1).to_i(8).chr }
      .gsub(/\\([()\\])/) { Regexp.last_match(1) }
  end
end

RSpec.configure do |config|
  config.include PdfTextExtractor
end
