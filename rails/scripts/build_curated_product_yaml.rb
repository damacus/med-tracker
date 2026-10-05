#!/usr/bin/env ruby
# frozen_string_literal: true

require 'optparse'
require 'yaml'

class CuratedProductYamlBuilder
  DEFAULT_SYSTEM = 'Curated product catalog'
  DEFAULT_CONCEPT_CLASS = 'Food supplement'
  DEFAULT_DOSE_FREQUENCY = 'As directed'
  DEFAULT_MAX_DAILY_DOSES = 1
  DEFAULT_MIN_HOURS_BETWEEN_DOSES = 24
  DEFAULT_DOSE_CYCLE = 'daily'

  REQUIRED_FIELDS = %i[gtin display dose_amount dose_unit dose_description].freeze

  class << self
    def build(options)
      attrs = symbolize_keys(options)
      validate!(attrs)

      product = compact_string_values(
        'gtin' => attrs[:gtin],
        'code' => attrs[:code],
        'display' => attrs[:display],
        'system' => attrs.fetch(:system, DEFAULT_SYSTEM),
        'concept_class' => attrs.fetch(:concept_class, DEFAULT_CONCEPT_CLASS),
        'category' => attrs[:category],
        'description' => attrs[:product_description],
        'warnings' => attrs[:warnings]
      )

      product['suggested_doses'] = [dose(attrs)]
      product
    end

    def dump(product)
      YAML.dump('products' => [product]).delete_prefix("---\n")
    end

    def append!(path, product)
      existing = File.exist?(path) ? YAML.load_file(path) : {}
      existing = {} unless existing.is_a?(Hash)
      products = Array(existing['products'])

      raise ArgumentError, duplicate_message(product) if duplicate?(products, product)

      existing['products'] = products + [product]
      File.write(path, YAML.dump(existing))
    end

    private

    def dose(attrs)
      compact_string_values(
        'amount' => number(attrs[:dose_amount]),
        'unit' => attrs[:dose_unit],
        'frequency' => attrs.fetch(:dose_frequency, DEFAULT_DOSE_FREQUENCY),
        'description' => attrs[:dose_description],
        'default_for_adults' => attrs[:adult_default],
        'default_for_children' => attrs[:child_default],
        'default_max_daily_doses' => number(attrs.fetch(:max_daily_doses, DEFAULT_MAX_DAILY_DOSES)),
        'default_min_hours_between_doses' => number(
          attrs.fetch(:min_hours_between_doses, DEFAULT_MIN_HOURS_BETWEEN_DOSES)
        ),
        'default_dose_cycle' => attrs.fetch(:dose_cycle, DEFAULT_DOSE_CYCLE),
        'current_supply' => optional_number(attrs[:current_supply]),
        'reorder_threshold' => optional_number(attrs[:reorder_threshold])
      )
    end

    def validate!(attrs)
      missing = REQUIRED_FIELDS.select { |field| blank?(attrs[field]) }
      return if missing.empty?

      flags = missing.map { |field| "--#{field.to_s.tr('_', '-')}" }.join(', ')
      raise ArgumentError, "Missing required option(s): #{flags}"
    end

    def duplicate?(products, product)
      products.any? do |entry|
        same_present_value?(entry, product, 'gtin') ||
          same_present_value?(entry, product, 'code') ||
          same_present_value?(entry, product, 'display')
      end
    end

    def duplicate_message(product)
      identifiers = %w[gtin code display].filter_map do |key|
        value = product[key]
        "#{key}=#{value}" unless blank?(value)
      end

      "Curated product already exists (#{identifiers.join(', ')})"
    end

    def same_present_value?(entry, product, key)
      entry_value = entry[key]
      product_value = product[key]
      !blank?(entry_value) && !blank?(product_value) && entry_value.to_s == product_value.to_s
    end

    def compact_string_values(hash)
      hash.each_with_object({}) do |(key, value), compacted|
        next if blank?(value)

        compacted[key] = value.is_a?(String) ? value.strip : value
      end
    end

    def symbolize_keys(hash)
      hash.each_with_object({}) { |(key, value), result| result[key.to_sym] = value }
    end

    def optional_number(value)
      return nil if blank?(value)

      number(value)
    end

    def number(value)
      text = value.to_s.strip.tr(',', '.')
      raise ArgumentError, "Invalid number: #{value.inspect}" unless text.match?(/\A\d+(?:\.\d+)?\z/)

      text.include?('.') ? text.to_f : text.to_i
    end

    def blank?(value)
      value.nil? || value.to_s.strip.empty?
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    options = {
      system: CuratedProductYamlBuilder::DEFAULT_SYSTEM,
      concept_class: CuratedProductYamlBuilder::DEFAULT_CONCEPT_CLASS,
      dose_frequency: CuratedProductYamlBuilder::DEFAULT_DOSE_FREQUENCY,
      max_daily_doses: CuratedProductYamlBuilder::DEFAULT_MAX_DAILY_DOSES,
      min_hours_between_doses: CuratedProductYamlBuilder::DEFAULT_MIN_HOURS_BETWEEN_DOSES,
      dose_cycle: CuratedProductYamlBuilder::DEFAULT_DOSE_CYCLE
    }

    parser = OptionParser.new do |opts|
      opts.banner = <<~BANNER
        Usage:
          ruby scripts/build_curated_product_yaml.rb --gtin GTIN --display NAME \\
            --category Vitamin --product-description TEXT --warnings TEXT \\
            --dose-amount 2 --dose-unit gummy --dose-description "Children 3+ years" \\
            --dose-frequency Daily --child-default --current-supply 60 --reorder-threshold 14

        Add --append config/nhs_dmd_curated_products.yml to write into the curated product list.
      BANNER

      opts.on('--gtin GTIN', 'Barcode / GTIN') { |value| options[:gtin] = value }
      opts.on('--code CODE', 'Optional dm+d or catalog code') { |value| options[:code] = value }
      opts.on('--display NAME', 'Display name') { |value| options[:display] = value }
      opts.on('--system SYSTEM', 'Source system') { |value| options[:system] = value }
      opts.on('--concept-class CLASS', 'Concept class, such as Food supplement') do |value|
        options[:concept_class] = value
      end
      opts.on('--category CATEGORY', 'Medication category') { |value| options[:category] = value }
      opts.on('--product-description TEXT', 'Product description') { |value| options[:product_description] = value }
      opts.on('--warnings TEXT', 'Safety warnings') { |value| options[:warnings] = value }
      opts.on('--dose-amount NUMBER', 'Dose amount') { |value| options[:dose_amount] = value }
      opts.on('--dose-unit UNIT', 'Dose unit') { |value| options[:dose_unit] = value }
      opts.on('--dose-frequency TEXT', 'Dose frequency') { |value| options[:dose_frequency] = value }
      opts.on('--dose-description TEXT', 'Dose description') { |value| options[:dose_description] = value }
      opts.on('--adult-default', 'Mark dose as the adult default') { options[:adult_default] = true }
      opts.on('--child-default', 'Mark dose as the child default') { options[:child_default] = true }
      opts.on('--max-daily-doses NUMBER', 'Maximum dose events per day') { |value| options[:max_daily_doses] = value }
      opts.on('--min-hours-between-doses NUMBER', 'Minimum hours between dose events') do |value|
        options[:min_hours_between_doses] = value
      end
      opts.on('--dose-cycle CYCLE', 'Dose cycle: daily, weekly, monthly') { |value| options[:dose_cycle] = value }
      opts.on('--current-supply NUMBER', 'Initial stock quantity') { |value| options[:current_supply] = value }
      opts.on('--reorder-threshold NUMBER', 'Low-stock threshold') { |value| options[:reorder_threshold] = value }
      opts.on('--append PATH', 'Append to an existing curated products YAML file') { |value| options[:append] = value }
    end

    parser.parse!
    product = CuratedProductYamlBuilder.build(options)

    if options[:append]
      CuratedProductYamlBuilder.append!(options[:append], product)
      warn "Appended curated product #{product['display']} to #{options[:append]}"
    else
      puts CuratedProductYamlBuilder.dump(product)
    end
  rescue ArgumentError, OptionParser::ParseError => e
    warn e.message
    exit 1
  end
end
