#!/usr/bin/env ruby
# frozen_string_literal: true

# Searches OpenFoodFacts for vitamin and supplement products sold by UK
# supermarkets (Tesco, Morrisons, ASDA, Sainsbury's, Boots, Superdrug) and
# prints results suitable for review and entry into the curated product list.
#
# Usage:
#   ruby scripts/search_grocery_vitamins.rb
#   ruby scripts/search_grocery_vitamins.rb --brands tesco,asda
#   ruby scripts/search_grocery_vitamins.rb --brands morrisons --query "children multivitamin"
#   ruby scripts/search_grocery_vitamins.rb --barcode 5057753926137
#   ruby scripts/search_grocery_vitamins.rb --format yaml
#
# Pipe yaml output into build_curated_product_yaml.rb to add to the curated list.

require 'json'
require 'net/http'
require 'optparse'
require 'uri'
require 'yaml'

module GroceryVitaminSearch
  BASE_URL = 'https://world.openfoodfacts.org'
  USER_AGENT = 'MedTracker/1.0 (support@medtracker.app)'
  TIMEOUT_SECONDS = 10
  PAGE_SIZE = 20

  SUPERMARKET_BRANDS = %w[tesco morrisons asda sainsburys boots superdrug].freeze

  VITAMIN_CATEGORY_KEYWORDS = %w[
    vitamin supplement mineral multivitamin omega fish-oil probiotic
    folic-acid iron zinc magnesium calcium
  ].freeze

  class Client
    def barcode_lookup(barcode)
      normalized = barcode.to_s.gsub(/\D/, '').rjust(13, '0')
      uri = URI("#{BASE_URL}/api/v2/product/#{normalized}.json")
      uri.query = URI.encode_www_form('fields' => detail_fields.join(','))
      payload = fetch_json(uri)
      return nil unless payload.is_a?(Hash) && payload['status'] == 1

      payload['product']&.merge('code' => normalized)
    end

    def search_by_brand(brand, query: nil, page: 1)
      uri = URI("#{BASE_URL}/cgi/search.pl")
      uri.query = URI.encode_www_form(brand_search_params(brand, query, page))
      payload = fetch_json(uri)
      Array(payload.is_a?(Hash) ? payload['products'] : [])
    end

    def brand_search_params(brand, query, page)
      params = {
        'action' => 'process',
        'json' => '1',
        'page_size' => PAGE_SIZE.to_s,
        'page' => page.to_s,
        'tagtype_0' => 'brands',
        'tag_contains_0' => 'contains',
        'tag_0' => brand,
        'tagtype_1' => 'countries',
        'tag_contains_1' => 'contains',
        'tag_1' => 'united-kingdom',
        'fields' => search_fields.join(',')
      }
      params['search_terms'] = query if query
      params
    end

    private

    def search_fields
      %w[code product_name brands quantity categories_tags]
    end

    def detail_fields
      %w[code product_name brands generic_name quantity categories_tags image_url]
    end

    def fetch_json(uri)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = true
      http.open_timeout = TIMEOUT_SECONDS
      http.read_timeout = TIMEOUT_SECONDS
      request = Net::HTTP::Get.new(uri)
      request['Accept'] = 'application/json'
      request['User-Agent'] = USER_AGENT
      response = http.request(request)
      JSON.parse(response.body)
    rescue StandardError => e
      warn "  Warning: request failed — #{e.message}"
      {}
    end
  end

  class ResultFilter
    def vitamin?(product)
      categories = Array(product['categories_tags'])
      categories.any? { |cat| VITAMIN_CATEGORY_KEYWORDS.any? { |kw| cat.include?(kw) } }
    end
  end

  class Formatter
    def format_table(products)
      return "No results found.\n" if products.empty?

      rows = products.map { |p| table_row(p) }
      col_widths = column_widths(rows)
      header = format_row(%w[GTIN Brand Name Qty], col_widths)
      separator = col_widths.map { |w| '-' * w }.join('  ')
      "#{([header, separator] + rows.map { |r| format_row(r, col_widths) }).join("\n")}\n"
    end

    def format_yaml_hints(products)
      return "# No results found.\n" if products.empty?

      products.map { |p| yaml_hint(p) }.join("\n")
    end

    private

    def table_row(product)
      [
        product['code'].to_s,
        truncate(Array(product['brands']).first.to_s, 15),
        truncate(product['product_name'].to_s, 50),
        product['quantity'].to_s
      ]
    end

    def yaml_hint(product)
      gtin = product['code'].to_s
      name = product['product_name'].to_s.strip
      qty  = product['quantity'].to_s.strip

      build_yaml_hint_lines(gtin, name, qty).join("\n")
    end

    def build_yaml_hint_lines(gtin, name, qty)
      [
        "# #{name} (#{qty})",
        '# ruby scripts/build_curated_product_yaml.rb \\',
        "#   --gtin #{gtin} \\",
        "#   --display #{name.inspect} \\",
        '#   --category Vitamin \\',
        "#   --dose-amount 1 --dose-unit tablet --dose-description 'Adults' \\",
        '#   --current-supply TBD --reorder-threshold TBD \\',
        '#   --append config/nhs_dmd_curated_products.yml'
      ]
    end

    def column_widths(rows)
      return [13, 15, 50, 10] if rows.empty?

      rows.each_with_object([0, 0, 0, 0]) do |row, widths|
        row.each_with_index { |cell, i| widths[i] = [widths[i], cell.length].max }
      end
    end

    def format_row(cells, widths)
      cells.each_with_index.map { |cell, i| cell.ljust(widths[i]) }.join('  ')
    end

    def truncate(str, max)
      str.length > max ? "#{str[0, max - 1]}…" : str
    end
  end

  class Runner
    def initialize(brands:, query:, barcode:, format:)
      @brands  = brands
      @query   = query
      @barcode = barcode
      @format  = format
      @client  = Client.new
      @filter  = ResultFilter.new
      @formatter = Formatter.new
    end

    def run
      products = @barcode ? lookup_barcode : search_brands
      print_results(products)
    end

    private

    def lookup_barcode
      warn "Looking up barcode #{@barcode}..."
      product = @client.barcode_lookup(@barcode)
      product ? [product] : []
    end

    def search_brands
      results = []
      @brands.each do |brand|
        query_suffix = @query ? " for #{@query.inspect}" : ''
        warn "Searching #{brand}#{query_suffix}..."
        products = @client.search_by_brand(brand, query: @query)
        vitamins = products.select { |p| @filter.vitamin?(p) }
        warn "  #{vitamins.size} vitamin/supplement products found (#{products.size} total)"
        results.concat(vitamins)
      end
      results.uniq { |p| p['code'] }
    end

    def print_results(products)
      case @format
      when 'yaml'
        puts @formatter.format_yaml_hints(products)
      else
        puts @formatter.format_table(products)
        puts "\nRun with --format yaml to see build_curated_product_yaml.rb invocations."
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  options = {
    brands: GroceryVitaminSearch::SUPERMARKET_BRANDS,
    query: nil,
    barcode: nil,
    format: 'table'
  }

  OptionParser.new do |opts|
    opts.banner = <<~BANNER
      Search OpenFoodFacts for UK supermarket vitamin and supplement products.

      Usage:
        ruby scripts/search_grocery_vitamins.rb [options]

      Examples:
        ruby scripts/search_grocery_vitamins.rb
        ruby scripts/search_grocery_vitamins.rb --brands tesco,morrisons
        ruby scripts/search_grocery_vitamins.rb --brands asda --query "children multivitamin"
        ruby scripts/search_grocery_vitamins.rb --barcode 5057753926137
        ruby scripts/search_grocery_vitamins.rb --format yaml
    BANNER

    opts.on('--brands BRANDS', 'Comma-separated list of supermarket brands to search',
            "(default: #{GroceryVitaminSearch::SUPERMARKET_BRANDS.join(',')})") do |v|
      options[:brands] = v.split(',').map(&:strip).reject(&:empty?)
    end

    opts.on('--query QUERY', 'Optional search query to narrow results') do |v|
      options[:query] = v
    end

    opts.on('--barcode BARCODE', 'Look up a specific product by barcode/GTIN') do |v|
      options[:barcode] = v
    end

    opts.on('--format FORMAT', 'Output format: table (default) or yaml') do |v|
      options[:format] = v
    end

    opts.on('-h', '--help', 'Show this help') do
      puts opts
      exit
    end
  end.parse!

  GroceryVitaminSearch::Runner.new(
    brands: options[:brands],
    query: options[:query],
    barcode: options[:barcode],
    format: options[:format]
  ).run
end
