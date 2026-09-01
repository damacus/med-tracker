require 'nokogiri'
require 'sghtmltopdf'
require 'uri'

module Reports
  class PdfRenderer
    class Error < StandardError
    end

    FONT_DIRECTORY = Rails.root.join('vendor/fonts').freeze

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
      html_assets = document.css(
        'img[src], source[src], video[src], audio[src], embed[src], iframe[src], object[data]'
      )
      stylesheet_links = document.css('link[rel~="stylesheet"][href], link[rel~="icon"][href]')

      html_assets.filter_map { |element| element['src'] || element['data'] } +
        stylesheet_links.filter_map { |element| element['href'] } +
        css_asset_references(document)
    end

    def css_asset_references(document)
      document.css('style, [style]').flat_map do |element|
        element.text.scan(/url\(([^)]+)\)/i).flatten.map do |reference|
          reference.delete_prefix('"').delete_suffix('"').delete_prefix("'").delete_suffix("'")
        end
      end
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
