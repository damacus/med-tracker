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

RSpec.describe OpenapiRouteCoverage do
  it 'documents every mounted API v1 route' do
    described_class.api_route_operations.each do |path, verb|
      expect(described_class.mounted_paths).to include(path)
      expect(described_class.mounted_paths.fetch(path)).to include(verb.downcase)
    end
  end
end
