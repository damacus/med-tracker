require 'nokogiri'
require 'sghtmltopdf'
require 'uri'

module Reports
  class PdfRenderer
    class Error < StandardError
    end

    FONT_DIRECTORY = Rails.root.join('vendor/fonts').freeze
    CSS_URL_PATTERN = /url\(\s*(?:(['"])(.*?)\1|([^)'"\s]+))\s*\)/i.freeze

    def render(component:, metadata:)
      configure_renderer
      html = component.call
      ensure_permitted_assets!(html)

      Sghtmltopdf.render(html, page_size: 'A4', **pdf_metadata(metadata))
    rescue Sghtmltopdf::Error => error
      raise Error, "PDF rendering failed: #{error.class}"
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
      stylesheets = document.css('style').map(&:text) + document.css('[style]').filter_map { |element| element['style'] }

      stylesheets.each do |stylesheet|
        raise Error, 'PDF assets must be bundled fonts' if css_import_at_keyword?(stylesheet)
      end

      stylesheets.flat_map { |stylesheet| css_url_references(stylesheet) }
    end

    def css_url_references(stylesheet)
      stylesheet.scan(CSS_URL_PATTERN).filter_map { |_quote, quoted, unquoted| quoted || unquoted }
    end

    def css_import_at_keyword?(stylesheet)
      index = 0

      while index < stylesheet.length
        if stylesheet[index, 2] == '/*'
          closing_comment = stylesheet.index('*/', index + 2)
          return false unless closing_comment

          index = closing_comment + 2
        elsif stylesheet[index] == "'" || stylesheet[index] == '"'
          index = css_string_end(stylesheet, index)
        elsif stylesheet[index] == '@'
          identifier, index = css_identifier_at(stylesheet, index + 1)
          return true if identifier.casecmp?('import')
        else
          index += 1
        end
      end

      false
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
        character = stylesheet[index]
        if character.match?(/[a-zA-Z0-9_-]/)
          identifier << character
          index += 1
        elsif character == '\\'
          escaped_character, index = css_escape_at(stylesheet, index)
          break unless escaped_character

          identifier << escaped_character
        else
          break
        end
      end

      [identifier, index]
    end

    def css_escape_at(stylesheet, index)
      index += 1
      return [nil, index] if index >= stylesheet.length
      return [nil, index] if stylesheet[index].match?(/[\n\r\f]/)

      escaped = stylesheet[index, 6][/\A[0-9a-fA-F]{1,6}/]
      return [stylesheet[index], index + 1] unless escaped

      index += escaped.length
      index += 1 if stylesheet[index]&.match?(/[ \t\n\r\f]/)

      codepoint = escaped.to_i(16)
      character = codepoint.zero? || codepoint > 0x10ffff ? "\uFFFD" : codepoint.chr(Encoding::UTF_8)

      [character, index]
    end

    def permitted_local_asset?(reference)
      uri = URI.parse(reference)
      return false unless uri.scheme.nil? || uri.scheme == 'file'
      return false if uri.host.present?

      path = uri.scheme == 'file' ? Pathname.new(uri.path) : FONT_DIRECTORY.join(uri.path)
      path = Pathname.new(File.expand_path(path.to_s))

      path.to_s.start_with?("#{FONT_DIRECTORY}/")
    end
  end
end
