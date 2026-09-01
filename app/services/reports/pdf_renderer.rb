require 'nokogiri'
require 'sghtmltopdf'
require 'uri'

module Reports
  class PdfRenderer
    class Error < StandardError
    end

    FONT_DIRECTORY = Rails.root.join('vendor/fonts').freeze
    CSS_URL_PATTERN = /url\(\s*(?:(['"])(.*?)\1|([^)'"\s]+))\s*\)/i

    def render(component:, metadata:)
      configure_renderer
      html = component.call
      ensure_permitted_assets!(html)

      Sghtmltopdf.render(html, page_size: 'A4', **pdf_metadata(metadata))
    rescue Sghtmltopdf::Error => e
      raise Error, "PDF rendering failed: #{e.class}"
    end

    private

    def configure_renderer
      Sghtmltopdf.configure do |config|
        config.page_size = 'A4'
        config.gothic_font = FONT_DIRECTORY.join('NotoSans-Regular.ttf').to_s
        config.base_url = FONT_DIRECTORY.to_s
        config.allow = [FONT_DIRECTORY.to_s]
      end
    end

    def pdf_metadata(metadata)
      metadata.slice(:title, :author, :subject, :keywords).compact
    end

    def ensure_permitted_assets!(html)
      asset_references(html).each do |reference|
        raise Error, 'PDF assets must be bundled fonts' unless permitted_local_asset?(reference)
      end
    rescue URI::InvalidURIError
      raise Error, 'PDF assets must be bundled fonts'
    end

    def asset_references(html)
      document = Nokogiri::HTML5.fragment(html)
      resource_attribute_references(document) + css_asset_references(document)
    end

    def resource_attribute_references(document)
      references = []

      document.traverse do |element|
        next unless element.element?

        element.attribute_nodes.each do |attribute|
          references << attribute.value if resource_attribute?(element, attribute)
        end
      end

      references
    end

    def resource_attribute?(element, attribute)
      return true if %w[src srcset poster background data].include?(attribute.name)

      attribute.name.end_with?('href') && element.name != 'a'
    end

    def css_asset_references(document)
      stylesheets = style_blocks(document) + inline_styles(document)

      stylesheets.each do |stylesheet|
        raise Error, 'PDF assets must be bundled fonts' if css_import_at_keyword?(stylesheet)
      end

      stylesheets.flat_map { |stylesheet| css_url_references(stylesheet) }
    end

    def css_url_references(stylesheet)
      stylesheet.scan(CSS_URL_PATTERN).filter_map { |_quote, quoted, unquoted| quoted || unquoted }
    end

    def css_import_at_keyword?(stylesheet)
      css_tokens(stylesheet).include?(:import)
    end

    def style_blocks(document)
      document.css('style').map(&:text)
    end

    def inline_styles(document)
      document.css('[style]').filter_map { |element| element['style'] }
    end

    def css_tokens(stylesheet)
      index = 0
      tokens = []

      while index < stylesheet.length
        token, index = css_token_at(stylesheet, index)
        tokens << token if token
      end

      tokens
    end

    def css_token_at(stylesheet, index)
      return css_comment_at(stylesheet, index) if stylesheet[index, 2] == '/*'
      return [nil, css_string_end(stylesheet, index)] if quote?(stylesheet[index])
      return css_at_keyword_at(stylesheet, index) if stylesheet[index] == '@'

      [nil, index + 1]
    end

    def css_comment_at(stylesheet, index)
      closing_comment = stylesheet.index('*/', index + 2)
      return [nil, stylesheet.length] unless closing_comment

      [nil, closing_comment + 2]
    end

    def quote?(character)
      ["'", '"'].include?(character)
    end

    def css_at_keyword_at(stylesheet, index)
      identifier, next_index = css_identifier_at(stylesheet, index + 1)
      [identifier.casecmp?('import') ? :import : nil, next_index]
    end

    def css_string_end(stylesheet, index)
      quote = stylesheet[index]
      index += 1

      while index < stylesheet.length
        if stylesheet[index] == '\\'
          index += 2
        elsif stylesheet[index] == quote
          return index + 1
        else
          index += 1
        end
      end

      index
    end

    def css_identifier_at(stylesheet, index)
      identifier = String.new

      while index < stylesheet.length
        character, next_index = css_identifier_character_at(stylesheet, index)
        break unless character

        identifier << character
        index = next_index
      end

      [identifier, index]
    end

    def css_identifier_character_at(stylesheet, index)
      character = stylesheet[index]
      return [character, index + 1] if character.match?(/[a-zA-Z0-9_-]/)
      return css_escape_at(stylesheet, index) if character == '\\'

      [nil, index]
    end

    def css_escape_at(stylesheet, index)
      character = stylesheet[index + 1]
      return [nil, index + 1] unless escaped_character?(character)

      css_escape_character_at(stylesheet, index + 1)
    end

    def escaped_character?(character)
      character && !character.match?(/[\n\r\f]/)
    end

    def css_escape_character_at(stylesheet, index)
      escaped = stylesheet[index, 6][/\A[0-9a-fA-F]{1,6}/]
      return [stylesheet[index], index + 1] unless escaped

      css_hex_escape_at(stylesheet, index, escaped)
    end

    def css_hex_escape_at(stylesheet, index, escaped)
      next_index = index + escaped.length
      next_index += 1 if stylesheet[next_index]&.match?(/[ \t\n\r\f]/)

      codepoint = escaped.to_i(16)
      character = codepoint.zero? || codepoint > 0x10ffff ? "\uFFFD" : codepoint.chr(Encoding::UTF_8)

      [character, next_index]
    end

    def permitted_local_asset?(reference)
      uri = URI.parse(reference)
      return false unless local_file_uri?(uri)

      bundled_font_path(uri).to_s.start_with?("#{FONT_DIRECTORY}/")
    end

    def local_file_uri?(uri)
      (uri.scheme.nil? || uri.scheme == 'file') && uri.host.blank?
    end

    def bundled_font_path(uri)
      path = uri.scheme == 'file' ? Pathname.new(uri.path) : FONT_DIRECTORY.join(uri.path)

      Pathname.new(File.expand_path(path.to_s))
    end
  end
end
