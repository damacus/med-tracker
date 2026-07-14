# frozen_string_literal: true

require 'rails_helper'

module OpenapiRouteCoverage
  module_function

  def document
    YAML.safe_load(Rails.root.join('docs/api/openapi.v1.yaml').read)
  end

  def paths
    document.fetch('paths')
  end

  def mounted_paths
    server_url = document.fetch('servers').first.fetch('url')

    paths.to_h { |path, path_item| ["#{server_url}#{path}", path_item] }
  end

  def api_route_operations
    Rails.application.routes.routes.filter_map do |route|
      next unless route.defaults.fetch(:controller, '').start_with?('api/v1/')

      [openapi_path(route), route.verb]
    end.uniq
  end

  def openapi_path(route)
    route.path.spec.to_s
         .delete_suffix('(.:format)')
         .gsub(/:(\w+)/, '{\1}')
  end
end

module OpenapiStructure
  HTTP_METHODS = %w[delete get head options patch post put trace].freeze
  AUDIENCE_TAGS = ['Public', 'Account', 'Household', 'Household administration'].freeze

  module_function

  def document
    YAML.safe_load(Rails.root.join('docs/api/openapi.v1.yaml').read)
  end

  def paths
    document.fetch('paths')
  end

  def operations
    paths.flat_map do |path, path_item|
      path_item.filter_map do |method, operation|
        [path, method, operation] if HTTP_METHODS.include?(method)
      end
    end
  end

  def defined_tags
    document.fetch('tags')
  end
end

RSpec.describe OpenapiRouteCoverage do
  it 'documents every mounted API v1 route' do
    described_class.api_route_operations.each do |path, verb|
      expect(described_class.mounted_paths).to include(path)
      expect(described_class.mounted_paths.fetch(path)).to include(verb.downcase)
    end
  end

  describe OpenapiStructure do
    it 'uses the canonical API v1 server address' do
      expect(described_class.document.fetch('servers').first.fetch('url')).to eq('/api/v1')
    end

    it 'uses normalized paths that compose with the API v1 server once' do
      described_class.paths.each_key do |path|
        expect(path).to start_with('/')
        expect(path).not_to start_with('/api/v1')
        expect("/api/v1#{path}").not_to include('/api/v1/api/v1')
      end
    end

    it 'gives every operation a unique lower-camel-case operation ID' do
      operation_ids = described_class.operations.map { |_path, _method, operation| operation['operationId'] }

      expect(operation_ids).to all(match(/\A[a-z][A-Za-z0-9]*\z/))
      expect(operation_ids).to all(be_present)
      expect(operation_ids).to eq(operation_ids.uniq)
    end

    it 'gives every operation one audience tag and one resource tag' do
      described_class.operations.each do |_path, _method, operation|
        tags = operation.fetch('tags')
        audience_tags = tags & described_class::AUDIENCE_TAGS

        expect(tags.size).to eq(2)
        expect(audience_tags.size).to eq(1)
        expect((tags - described_class::AUDIENCE_TAGS).size).to eq(1)
      end
    end

    it 'defines every used tag once with a description' do
      defined_tags = described_class.defined_tags
      defined_tag_names = defined_tags.map { |tag| tag.fetch('name') }
      used_tag_names = described_class.operations.flat_map { |_path, _method, operation| operation.fetch('tags') }.uniq

      expect(defined_tag_names).to eq(defined_tag_names.uniq)
      expect(defined_tags).to all(include('description'))
      expect(defined_tags.map { |tag| tag.fetch('description') }).to all(be_present)
      expect(defined_tag_names).to match_array(used_tag_names)
    end
  end
end
