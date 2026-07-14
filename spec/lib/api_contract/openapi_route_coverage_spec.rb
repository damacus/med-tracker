# frozen_string_literal: true

require 'rails_helper'
require 'json_schemer'

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

module OpenapiYamlValidation
  def duplicate_mapping_keys(node = Psych.parse(source).root, path = '#')
    return mapping_duplicate_keys(node, path) if node.is_a?(Psych::Nodes::Mapping)
    return sequence_duplicate_keys(node, path) if node.is_a?(Psych::Nodes::Sequence)

    []
  end

  def mapping_duplicate_keys(node, path)
    entries = node.children.each_slice(2).to_a
    duplicate_key_paths(entries, path) + entries.flat_map do |key, value|
      duplicate_mapping_keys(value, "#{path}/#{key.value}")
    end
  end

  def sequence_duplicate_keys(node, path)
    node.children.each_with_index.flat_map do |child, index|
      duplicate_mapping_keys(child, "#{path}/#{index}")
    end
  end

  def duplicate_key_paths(entries, path)
    entries.map { |key, _value| key.value }.tally.filter_map do |key, count|
      "#{path}/#{key}" if count > 1
    end
  end

  def unsupported_free_form_errors(value = document, path = '#')
    errors = unsupported_free_form?(value, path) ? [path] : []
    errors + nested_free_form_errors(value, path)
  end

  def nested_free_form_errors(value, path)
    case value
    when Hash
      value.flat_map { |key, child| unsupported_free_form_errors(child, "#{path}/#{key}") }
    when Array
      value.each_with_index.flat_map { |child, index| unsupported_free_form_errors(child, "#{path}/#{index}") }
    else
      []
    end
  end

  def unsupported_free_form?(value, path)
    value.is_a?(Hash) && value['type'] == 'object' && value['additionalProperties'] == true &&
      OpenapiStructure::ALLOWED_FREE_FORM_PATHS.exclude?(path)
  end
end

module OpenapiReferenceValidation
  def local_reference_errors(value = document, path = '#', root = document)
    errors = broken_local_reference?(value, root) ? ["#{path}/$ref"] : []
    errors + nested_reference_errors(value, path, root)
  end

  def nested_reference_errors(value, path, root)
    case value
    when Hash
      value.flat_map { |key, child| local_reference_errors(child, "#{path}/#{key}", root) }
    when Array
      value.each_with_index.flat_map { |child, index| local_reference_errors(child, "#{path}/#{index}", root) }
    else
      []
    end
  end

  def broken_local_reference?(value, root)
    value.is_a?(Hash) && value.key?('$ref') && resolve_local_reference(value.fetch('$ref'), root:).nil?
  end

  def resolve_local_reference(reference, root: document)
    return unless reference.start_with?('#/')

    resolve_reference_tokens(root, reference.delete_prefix('#/').split('/'))
  end

  def resolve_reference_tokens(node, tokens)
    return node if tokens.empty?
    return unless node.is_a?(Hash)

    token, *remaining = tokens
    resolve_reference_tokens(node[token.gsub('~1', '/').gsub('~0', '~')], remaining)
  end

  def dereferenced_schema(name)
    dereference(schema(name))
  end

  def dereference(value, root: document)
    case value
    when Hash
      return dereference(resolve_local_reference(value.fetch('$ref'), root:), root:) if value.key?('$ref')

      value.transform_values { |child| dereference(child, root:) }
    when Array
      value.map { |child| dereference(child, root:) }
    else
      value
    end
  end

  def schema_errors(name, payload)
    JSONSchemer.schema(dereferenced_schema(name)).validate(payload.deep_stringify_keys).map do |error|
      error.fetch('data_pointer')
    end
  end
end

module OpenapiStructure
  HTTP_METHODS = %w[delete get head options patch post put trace].freeze
  AUDIENCE_TAGS = ['Public', 'Account', 'Household', 'Household administration'].freeze
  ALLOWED_FREE_FORM_PATHS = ['#/components/schemas/SyncBatchOperation/properties/attributes'].freeze

  extend OpenapiYamlValidation
  extend OpenapiReferenceValidation

  module_function

  def document
    YAML.safe_load(Rails.root.join('docs/api/openapi.v1.yaml').read)
  end

  def source
    Rails.root.join('docs/api/openapi.v1.yaml').read
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

  def components
    document.fetch('components')
  end

  def schema(name)
    components.fetch('schemas').fetch(name)
  end

  def operation(path, method)
    paths.fetch(path).fetch(method)
  end
end

RSpec.describe OpenapiRouteCoverage, type: :request do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :carer_relationships

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

    it 'defines distinct identifiers, timestamps, and pagination' do
      schemas = described_class.components.fetch('schemas')

      expect(schemas.fetch('NumericId')).to include('type' => 'integer', 'minimum' => 1)
      expect(schemas.fetch('PortableId')).to include('type' => 'string', 'format' => 'uuid')
      expect(schemas.fetch('ResourceIdentifier').fetch('oneOf')).to contain_exactly(
        { '$ref' => '#/components/schemas/NumericId' },
        { '$ref' => '#/components/schemas/PortableId' }
      )
      expect(schemas.fetch('Timestamp')).to include('type' => 'string', 'format' => 'date-time')
      expect(schemas.fetch('PaginationMeta').fetch('required')).to contain_exactly(
        'page', 'per_page', 'total_count'
      )
    end

    it 'uses shared path, precondition, and response header components' do
      parameters = described_class.components.fetch('parameters')
      headers = described_class.components.fetch('headers')

      expect(parameters.fetch('id').dig('schema', '$ref')).to eq('#/components/schemas/ResourceIdentifier')
      expect(parameters.fetch('household_id').dig('schema', '$ref')).to eq('#/components/schemas/NumericId')
      expect(parameters.fetch('if_match').fetch('in')).to eq('header')
      expect(headers.fetch('etag').dig('schema', 'type')).to eq('string')
    end

    it 'defines shared errors and bearer security' do
      responses = described_class.components.fetch('responses')

      expect(responses).to include(
        'Unauthorized', 'Forbidden', 'NotFound', 'ValidationFailed', 'Conflict',
        'PreconditionRequired', 'RateLimited'
      )
      expect(described_class.components.dig('securitySchemes', 'bearerAuth')).to include(
        'type' => 'http', 'scheme' => 'bearer'
      )
    end

    it 'models intentional unauthenticated exceptions without weakening protected operations' do
      unauthenticated = described_class.operations.filter_map do |path, method, operation|
        "#{method.upcase} #{path}" if operation['security'] == []
      end

      expect(described_class.document.fetch('security')).to eq([{ 'bearerAuth' => [] }])
      expect(unauthenticated).to contain_exactly(
        'GET /capabilities',
        'POST /auth/login',
        'POST /auth/oidc_exchange',
        'POST /auth/refresh'
      )
      expect(described_class.operation('/auth/logout', 'delete')).not_to include('security')
    end

    it 'validates OpenAPI structure, local references, and object boundaries' do
      expect(described_class.document).to include('openapi' => '3.1.0')
      expect(described_class.document).to include('info', 'servers', 'paths', 'components')
      expect(described_class.duplicate_mapping_keys).to be_empty
      expect(described_class.local_reference_errors).to be_empty
      expect(described_class.unsupported_free_form_errors).to be_empty
    end

    it 'loads every reusable schema through the JSON Schema validator' do
      expect do
        described_class.components.fetch('schemas').each_key do |name|
          JSONSchemer.schema(described_class.dereferenced_schema(name))
        end
      end.not_to raise_error
    end

    it 'matches a representative Rails request, response, and nullable fields' do
      user = users(:jane)
      login_data = api_login(user)
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))
      request_body = {
        'person' => {
          'name' => 'API Contract Dependent',
          'date_of_birth' => 8.years.ago.to_date.iso8601,
          'person_type' => 'minor'
        }
      }

      expect(described_class.schema_errors('PersonCreateRequest', request_body)).to be_empty

      post api_v1_household_people_path(household_id), params: request_body, headers:, as: :json

      expect(response).to have_http_status(:created)
      expect(described_class.schema_errors('PersonResponse', response.parsed_body)).to be_empty
    end

    it 'matches a representative Rails validation error' do
      login_data = api_login(users(:jane))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      patch api_v1_household_person_path(household_id, people(:child_patient)),
            params: { person: { name: '' } }, headers:, as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(described_class.schema_errors('ErrorEnvelope', response.parsed_body)).to be_empty
    end

    it 'references typed representative operation schemas and shared errors' do
      create_person = described_class.operation('/households/{household_id}/people', 'post')
      show_person = described_class.operation('/households/{household_id}/people/{id}', 'get')
      update_person = described_class.operation('/households/{household_id}/people/{id}', 'patch')

      expect(create_person.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonCreateRequest'
      )
      expect(create_person.dig('responses', '201', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonResponse'
      )
      expect(show_person.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonResponse'
      )
      expect(update_person.dig('responses', '409', '$ref')).to eq('#/components/responses/Conflict')
      expect(update_person.dig('responses', '422', '$ref')).to eq('#/components/responses/ValidationFailed')
    end

    it 'reports schema failures by pointer without echoing response data' do
      invalid_payload = {
        'data' => {
          'id' => 'not-an-id',
          'name' => 'Private Patient Name',
          'email' => 'private@example.test'
        }
      }

      diagnostics = described_class.schema_errors('PersonResponse', invalid_payload).join(' ')

      expect(diagnostics).to include('/data')
      expect(diagnostics).not_to include('Private Patient Name', 'private@example.test', 'not-an-id')
    end
  end
end
