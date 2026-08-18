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
    value.is_a?(Hash) && value['type'] == 'object' &&
      (value['additionalProperties'] == true || !value.key?('additionalProperties')) &&
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
      dereference_hash(value, root:)
    when Array
      value.map { |child| dereference(child, root:) }
    else
      value
    end
  end

  def dereference_hash(value, root:)
    return dereference(resolve_local_reference(value.fetch('$ref'), root:), root:) if value.key?('$ref')

    dereferenced = value.transform_values { |child| dereference(child, root:) }
    return dereferenced unless dereferenced.delete('nullable')

    type = dereferenced['type']
    dereferenced['type'] = [type, 'null'] if type.is_a?(String)
    dereferenced['enum'] = [*dereferenced['enum'], nil] if dereferenced['enum'].is_a?(Array)
    dereferenced
  end

  def schema_errors(name, payload)
    JSONSchemer.schema(dereferenced_schema(name)).validate(payload.deep_stringify_keys).map do |error|
      error.fetch('data_pointer')
    end
  end
end

module OpenapiClientCompatibility
  CLIENT_LANGUAGE_KEYWORDS = %w[
    associatedtype as async await break borrow case catch class consuming continue deinit delegate do dynamic else enum
    expect extension external false field fileprivate finally for func fun get guard if import in infix init inout
    interface internal is let nil noinline not null object operator out override package param private protocol public
    reified repeat return self Self set static struct super suspend switch throw true try typealias typeof val var
    vararg where while
  ].freeze

  def client_schema_keyword_errors(value = document, path = '#')
    return hash_keyword_errors(value, path) if value.is_a?(Hash)
    return array_keyword_errors(value, path) if value.is_a?(Array)

    []
  end

  def hash_keyword_errors(value, path)
    keyword_errors(value, path) +
      inline_enum_keyword_errors(value, path) +
      value.flat_map { |key, child| client_schema_keyword_errors(child, "#{path}/#{key}") }
  end

  def keyword_errors(value, path)
    errors = %w[const oneOf anyOf].filter_map { |keyword| "#{path}/#{keyword}" if value.key?(keyword) }
    errors << "#{path}/type" if value['type'].is_a?(Array)
    errors + primitive_all_of_errors(value['allOf'], "#{path}/allOf")
  end

  def inline_enum_keyword_errors(value, path)
    properties = value['properties']
    return [] unless properties.is_a?(Hash)

    properties.filter_map do |property, property_schema|
      next unless CLIENT_LANGUAGE_KEYWORDS.include?(property)
      next unless property_schema.is_a?(Hash) && property_schema.key?('enum')

      "#{path}/properties/#{property}/enum"
    end
  end

  def array_keyword_errors(value, path)
    value.each_with_index.flat_map do |child, index|
      client_schema_keyword_errors(child, "#{path}/#{index}")
    end
  end

  def primitive_all_of_errors(value, path)
    return [] unless value.is_a?(Array)

    value.each_with_index.filter_map do |item, index|
      next unless item.is_a?(Hash) && item['type'].is_a?(String) && item['type'] != 'object'

      "#{path}/#{index}"
    end
  end
end

module OpenapiStructure
  HTTP_METHODS = %w[delete get head options patch post put trace].freeze
  AUDIENCE_TAGS = ['Public', 'Account', 'Household', 'Household administration'].freeze
  ALLOWED_FREE_FORM_PATHS = [
    '#/components/schemas/SecurityAuditEvent/properties/metadata',
    '#/components/schemas/SyncBatchOperation/properties/attributes',
    '#/components/schemas/SyncChange/properties/metadata',
    '#/components/schemas/SyncTombstone/properties/metadata',
    '#/components/schemas/PortableRecord'
  ].freeze
  LOCATOR_PATHS = %w[
    /households/{household_id}/locations/{id}
    /households/{household_id}/medications/{id}
    /households/{household_id}/medications/{id}/adjust_inventory
    /households/{household_id}/medications/{id}/mark_as_ordered
    /households/{household_id}/medications/{id}/mark_as_received
    /households/{household_id}/dosage_options/{id}
    /households/{household_id}/health_events/{id}
    /households/{household_id}/schedules/{id}
    /households/{household_id}/schedules/{id}/pause
    /households/{household_id}/schedules/{id}/resume
    /households/{household_id}/person_medications/{id}
    /households/{household_id}/person_medications/{id}/pause
    /households/{household_id}/person_medications/{id}/resume
    /households/{household_id}/person_medications/{id}/reorder
  ].freeze
  PRECONDITION_PATHS = %w[
    /households/{household_id}/medications/{id}
    /households/{household_id}/dosage_options/{id}
    /households/{household_id}/health_events/{id}
    /households/{household_id}/schedules/{id}
    /households/{household_id}/person_medications/{id}
  ].freeze
  OPENAPI_SCHEMA_PATH = Rails.root.join(
    'spec/fixtures/files/openapi-3.0-schema-2021-09-28.json'
  )

  extend OpenapiYamlValidation
  extend OpenapiReferenceValidation
  extend OpenapiClientCompatibility

  module_function

  def document
    YAML.safe_load(Rails.root.join('docs/api/openapi.v1.yaml').read)
  end

  def source = Rails.root.join('docs/api/openapi.v1.yaml').read

  def paths = document.fetch('paths')

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

  def document_schema_errors(openapi_document = document)
    schema = JSON.parse(OPENAPI_SCHEMA_PATH.read)
    JSONSchemer.schema(schema).validate(openapi_document).map { |error| error.fetch('data_pointer') }
  end

  def security_requirement_errors(openapi_document = document)
    defined_schemes = openapi_document.dig('components', 'securitySchemes').keys
    security_requirements(openapi_document).flat_map do |path, requirements|
      security_errors_for(path, requirements, defined_schemes)
    end
  end

  def security_errors_for(path, requirements, defined_schemes)
    return [path] unless requirements.is_a?(Array)

    requirements.flat_map do |requirement|
      errors = requirement.empty? && requirements.any? ? [path] : []
      errors + requirement.keys.filter_map do |scheme|
        "#{path}/#{scheme}" unless defined_schemes.include?(scheme)
      end
    end
  end

  def security_requirements(openapi_document)
    root = [['#/security', openapi_document.fetch('security')]]
    openapi_document.fetch('paths').each_with_object(root) do |(path, path_item), requirements|
      path_item.each do |method, operation|
        next unless HTTP_METHODS.include?(method) && operation.key?('security')

        pointer = "#/paths/#{path.gsub('/', '~1')}/#{method}/security"
        requirements << [pointer, operation.fetch('security')]
      end
    end
  end

  def unauthenticated_operations
    operations.filter_map do |path, method, operation|
      "#{method.upcase} #{path}" if operation['security'] == []
    end
  end

  def person_request_errors(attributes)
    schema_errors('PersonCreateRequest', 'person' => attributes)
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
    let(:person_attributes) do
      {
        'name' => 'API Contract Dependent',
        'date_of_birth' => 8.years.ago.to_date.iso8601,
        'person_type' => 'minor',
        'has_capacity' => true
      }
    end

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

    it 'defines distinct identifiers' do
      schemas = described_class.components.fetch('schemas')

      expect(schemas.fetch('NumericId')).to include('type' => 'integer', 'minimum' => 1)
      expect(schemas.fetch('PortableId')).to include('type' => 'string', 'format' => 'uuid')
      expect(schemas.fetch('ResourceIdentifier')).to include('type' => 'string')
      expect(schemas.fetch('ResourceIdentifier')).to include('pattern' => kind_of(String))
    end

    it 'defines decimal values as strings' do
      schemas = described_class.components.fetch('schemas')

      expect(schemas.fetch('DecimalValue')).to include('type' => 'string', 'pattern' => kind_of(String))
      expect(schemas.fetch('NullableDecimalValue')).to include('type' => 'string', 'nullable' => true)
    end

    it 'defines timestamps and pagination' do
      schemas = described_class.components.fetch('schemas')

      expect(schemas.fetch('Timestamp')).to include('type' => 'string', 'format' => 'date-time')
      expect(schemas.fetch('PaginationMeta').fetch('required')).to contain_exactly(
        'page', 'per_page', 'total_count'
      )
    end

    it 'uses OpenAPI 3.0.3 client-compatible schema composition' do
      expect(described_class.document.fetch('openapi')).to eq('3.0.3')
      expect(described_class.client_schema_keyword_errors).to be_empty
    end

    it 'rejects anonymous enums whose property names collide with client language keywords' do
      malformed_schema = {
        'type' => 'object',
        'properties' => {
          'class' => { 'type' => 'string', 'enum' => ['example'] }
        }
      }

      expect(described_class.client_schema_keyword_errors(malformed_schema, '#/Malformed')).to include(
        '#/Malformed/properties/class/enum'
      )
    end

    it 'keeps nullable enum values out of the enum declaration' do
      dose_cycle = described_class.schema('Schedule').dig('properties', 'dose_cycle')

      expect(dose_cycle).to include('type' => 'string', 'nullable' => true)
      expect(dose_cycle.fetch('enum')).to contain_exactly('daily', 'weekly', 'monthly')
    end

    it 'uses a reusable import conflict field enum' do
      expect(described_class.schema('PortableImportConflictField')).to include(
        'type' => 'string', 'enum' => %w[name email]
      )
      expect(described_class.schema('PortableImportConflict').dig('properties', 'field', '$ref')).to eq(
        '#/components/schemas/PortableImportConflictField'
      )
    end

    it 'uses shared path identifier components' do
      parameters = described_class.components.fetch('parameters')

      expect(parameters.fetch('id').dig('schema', '$ref')).to eq('#/components/schemas/NumericId')
      expect(parameters.fetch('resource_id').dig('schema', '$ref')).to eq(
        '#/components/schemas/ResourceIdentifier'
      )
      expect(parameters.fetch('native_device_token').dig('schema', '$ref')).to eq(
        '#/components/schemas/DeviceTokenIdentifier'
      )
      expect(parameters.fetch('household_id').dig('schema', '$ref')).to eq('#/components/schemas/NumericId')
    end

    it 'uses shared precondition and response header components' do
      parameters = described_class.components.fetch('parameters')
      headers = described_class.components.fetch('headers')

      expect(parameters.fetch('if_match').fetch('in')).to eq('header')
      expect(headers.fetch('etag').dig('schema', 'type')).to eq('string')
      expect(headers.fetch('etag')).to include('required' => true)
    end

    it 'uses portable-or-numeric identifiers on locator-backed resource paths' do
      described_class::LOCATOR_PATHS.each do |path|
        described_class.paths.fetch(path).each do |method, operation|
          next unless described_class::HTTP_METHODS.include?(method)

          expect(operation.fetch('parameters')).to include(
            { '$ref' => '#/components/parameters/resource_id' }
          )
        end
      end
    end

    it 'models ETag preconditions on every controller that enforces stale-write conflicts' do
      described_class::PRECONDITION_PATHS.each do |path|
        %w[patch put].each do |method|
          operation = described_class.operation(path, method)

          expect(operation.fetch('parameters')).to include({ '$ref' => '#/components/parameters/if_match' })
          expect(operation.dig('responses', '409', '$ref')).to eq('#/components/responses/Conflict')
          expect(operation.dig('responses', '200', 'headers', 'ETag', '$ref')).to eq(
            '#/components/headers/etag'
          )
        end
      end
    end

    it 'models ETag headers on reads that supply update preconditions' do
      described_class::PRECONDITION_PATHS.each do |path|
        expect(described_class.operation(path, 'get').dig('responses', '200', 'headers', 'ETag', '$ref')).to eq(
          '#/components/headers/etag'
        )
      end
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
      expect(described_class.document.fetch('security')).to eq([{ 'bearerAuth' => [] }])
      expect(described_class.security_requirement_errors).to be_empty
      expect(described_class.unauthenticated_operations).to contain_exactly(
        'GET /capabilities',
        'POST /auth/login',
        'POST /auth/oidc_exchange',
        'POST /auth/refresh'
      )
      expect(described_class.operation('/auth/logout', 'delete')).not_to include('security')
    end

    it 'rejects optional and unknown security requirements' do
      optional_security = described_class.document.deep_dup
      optional_security['paths']['/auth/logout']['delete']['security'] = [{}]
      unknown_security = described_class.document.deep_dup
      unknown_security['security'] = [{ 'unknownAuth' => [] }]

      expect(described_class.security_requirement_errors(optional_security)).to include(
        '#/paths/~1auth~1logout/delete/security'
      )
      expect(described_class.security_requirement_errors(unknown_security)).to include(
        '#/security/unknownAuth'
      )
    end

    it 'declares an OpenAPI 3.0.3 document with required sections' do
      expect(described_class.document).to include('openapi' => '3.0.3')
      expect(described_class.document).to include('info', 'servers', 'paths', 'components')
    end

    it 'rejects duplicate YAML mapping keys including keys inside sequences' do
      nested_duplicate = Psych.parse("items:\n  - name: first\n    name: second\n").root

      expect(described_class.duplicate_mapping_keys).to be_empty
      expect(described_class.duplicate_mapping_keys(nested_duplicate)).to eq(['#/items/0/name'])
    end

    it 'rejects broken references and unsupported free-form objects' do
      expect(described_class.local_reference_errors).to be_empty
      expect(described_class.unsupported_free_form_errors).to be_empty
      expect(described_class.unsupported_free_form_errors('type' => 'object')).to eq(['#'])
    end

    it 'validates the complete document against the pinned OpenAPI schema' do
      expect(described_class.document_schema_errors).to be_empty

      malformed_document = described_class.document.deep_dup
      malformed_document.fetch('info').delete('title')
      expect(described_class.document_schema_errors(malformed_document)).to include('/info')
    end

    it 'loads every reusable schema through the JSON Schema validator' do
      expect do
        described_class.components.fetch('schemas').each_key do |name|
          JSONSchemer.schema(described_class.dereferenced_schema(name))
        end
      end.not_to raise_error
    end

    it 'models person request fields according to Rails validation and defaults' do
      expect(described_class.person_request_errors(person_attributes)).to be_empty
      expect(described_class.person_request_errors(person_attributes.except('person_type'))).to be_empty
      expect(described_class.person_request_errors(person_attributes.merge('date_of_birth' => nil))).to include(
        '/person/date_of_birth'
      )
    end

    it 'matches a representative Rails create request and response' do
      login_data = api_login(users(:jane))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      post api_v1_household_people_path(household_id), params: { person: person_attributes }, headers:, as: :json

      expect(response).to have_http_status(:created)
      expect(response.headers['ETag']).to be_present
      expect(response.parsed_body.dig('data', 'has_capacity')).to be(false)
      expect(described_class.schema_errors('PersonResponse', response.parsed_body)).to be_empty
    end

    it 'allows negative serialized ages that Rails can currently emit' do
      person = Api::V1::PersonSerializer.new(people(:child_patient)).as_json.merge(age: -1)

      expect(described_class.schema_errors('PersonResponse', { data: person })).to be_empty
    end

    it 'allows nullable response fields emitted for legacy people' do
      person = Api::V1::PersonSerializer.new(people(:child_patient)).as_json.merge(date_of_birth: nil, age: nil)

      expect(described_class.schema_errors('PersonResponse', { data: person })).to be_empty
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

    it 'types the public capability response and no-store header' do
      operation = described_class.operation('/capabilities', 'get')

      expect(operation.fetch('security')).to eq([])
      expect(operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/CapabilitiesResponse'
      )
      expect(operation.dig('responses', '200', 'headers', 'Cache-Control', 'schema')).to include(
        'type' => 'string', 'enum' => ['no-store']
      )
    end

    it 'matches the public Rails capability payload without allowing drift' do
      get api_v1_capabilities_path, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('CapabilitiesResponse', response.parsed_body)).to be_empty
      expect(
        described_class.schema_errors(
          'CapabilitiesResponse',
          response.parsed_body.deep_merge('data' => { 'unexpected' => true })
        )
      ).to include('/data/unexpected')
    end

    it 'types household settings reads and updates' do
      path = '/households/{household_id}/admin/settings'
      get_operation = described_class.operation(path, 'get')
      write_operations = %w[patch put].map { |method| described_class.operation(path, method) }

      expect(get_operation.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(get_operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/HouseholdAdminSettingsResponse'
      )
      write_operations.each do |operation|
        expect(operation.fetch('responses').keys).to include('200', '400', '401', '403', '404', '409', '422', '429')
        expect(operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
          '#/components/schemas/HouseholdAdminSettingsUpdateRequest'
        )
      end
    end

    it 'matches the household settings Rails payload and fresh-proof error' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_admin_settings_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('HouseholdAdminSettingsResponse', response.parsed_body)).to be_empty

      patch api_v1_household_admin_settings_path(household_id),
            params: { household: { name: 'Contract update' } }, headers:, as: :json

      expect(response).to have_http_status(:forbidden)
      expect(described_class.schema_errors('AdminWriteForbiddenErrorEnvelope', response.parsed_body)).to be_empty
    end

    it 'rejects unsupported household settings request fields' do
      valid_request = { household: { name: 'Household', subscription_plan: 'family_plus' } }
      invalid_request = valid_request.deep_merge(household: { unexpected: true })

      expect(described_class.schema_errors('HouseholdAdminSettingsUpdateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('HouseholdAdminSettingsUpdateRequest', invalid_request)).to include(
        '/household/unexpected'
      )
    end

    it 'types household membership administration' do
      collection = described_class.operation('/households/{household_id}/admin/memberships', 'get')
      resource_path = '/households/{household_id}/admin/memberships/{id}'
      writes = %w[patch put delete].map { |method| described_class.operation(resource_path, method) }

      expect(collection.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(collection.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/HouseholdMembershipCollectionResponse'
      )
      expect(writes).to all(satisfy { |operation| operation.fetch('responses').key?('409') })
      expect(writes).to all(satisfy { |operation| operation.fetch('responses').key?('404') })
    end

    it 'matches the household membership Rails collection' do
      household_id, headers = manager_api_context

      get api_v1_household_admin_memberships_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('HouseholdMembershipCollectionResponse', response.parsed_body)).to be_empty
      expect(
        described_class.schema_errors(
          'HouseholdMembershipCollectionResponse', response.parsed_body.merge('meta' => {})
        )
      ).to include('/meta')
    end

    it 'rejects unsupported household membership request fields' do
      valid_request = { household_membership: { role: 'member', person_id: nil } }
      invalid_request = valid_request.deep_merge(household_membership: { unexpected: true })

      expect(described_class.schema_errors('HouseholdMembershipUpdateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('HouseholdMembershipUpdateRequest', invalid_request)).to include(
        '/household_membership/unexpected'
      )
    end

    it 'types household invitation administration' do
      collection_path = '/households/{household_id}/admin/invitations'
      collection = described_class.operation(collection_path, 'get')
      create = described_class.operation(collection_path, 'post')
      revoke = described_class.operation("#{collection_path}/{id}", 'delete')

      expect(collection.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(create.fetch('responses').keys).to include('201', '400', '401', '403', '404', '409', '422', '429')
      expect(revoke.fetch('responses').keys).to include('204', '401', '403', '404', '409', '429')
      expect(create.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/HouseholdInvitationCreateRequest'
      )
    end

    it 'matches household invitation Rails responses' do
      household_id, headers, session = manager_api_context
      session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

      post api_v1_household_admin_invitations_path(household_id),
           params: { household_invitation: { email: 'contract.invitation@example.test', membership_role: 'member' } },
           headers:, as: :json

      expect(response).to have_http_status(:created)
      expect(described_class.schema_errors('HouseholdInvitationResponse', response.parsed_body)).to be_empty

      get api_v1_household_admin_invitations_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('HouseholdInvitationCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'rejects invitation request fields that could expose secrets' do
      valid_request = { household_invitation: { email: 'invite@example.test', membership_role: 'member' } }
      invalid_request = valid_request.deep_merge(household_invitation: { token: 'private' })

      expect(described_class.schema_errors('HouseholdInvitationCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('HouseholdInvitationCreateRequest', invalid_request)).to include(
        '/household_invitation/token'
      )
    end

    it 'types person access grant administration' do
      collection_path = '/households/{household_id}/admin/person_access_grants'
      collection = described_class.operation(collection_path, 'get')
      create = described_class.operation(collection_path, 'post')
      revoke = described_class.operation("#{collection_path}/{id}", 'delete')

      expect(collection.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(create.fetch('responses').keys).to include('201', '400', '401', '403', '404', '409', '422', '429')
      expect(revoke.fetch('responses').keys).to include('204', '401', '403', '404', '409', '422', '429')
      expect(create.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonAccessGrantCreateRequest'
      )
    end

    it 'matches person access grant Rails responses' do
      household_id, headers, session = manager_api_context
      session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)
      membership = contract_membership(household_id)

      post api_v1_household_admin_person_access_grants_path(household_id),
           params: { person_access_grant: contract_grant_attributes(membership) }, headers:, as: :json

      expect(response).to have_http_status(:created)
      expect(described_class.schema_errors('PersonAccessGrantResponse', response.parsed_body)).to be_empty

      get api_v1_household_admin_person_access_grants_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('PersonAccessGrantCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'rejects unsupported person access grant request fields' do
      valid_request = { person_access_grant: contract_grant_attributes(contract_membership(manager_api_context.first)) }
      invalid_request = valid_request.deep_merge(person_access_grant: { unexpected: true })

      expect(described_class.schema_errors('PersonAccessGrantCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('PersonAccessGrantCreateRequest', invalid_request)).to include(
        '/person_access_grant/unexpected'
      )
    end

    it 'types API app token administration without advertising unsupported validation' do
      collection_path = '/households/{household_id}/admin/app_tokens'
      collection = described_class.operation(collection_path, 'get')
      create = described_class.operation(collection_path, 'post')
      revoke = described_class.operation("#{collection_path}/{id}", 'delete')

      expect(collection.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(create.fetch('responses').keys).to include('201', '400', '401', '403', '404', '409', '429')
      expect(create.fetch('responses')).not_to include('422')
      expect(revoke.fetch('responses').keys).to include('204', '401', '403', '404', '409', '429')
    end

    it 'matches API app token Rails responses without leaking listed tokens' do
      household_id, headers, session = manager_api_context
      session.update!(oidc_mfa_verified: true, mfa_verified_at: Time.current)

      post api_v1_household_admin_app_tokens_path(household_id),
           params: { api_app_token: { name: 'Contract token' } }, headers:, as: :json

      expect(response).to have_http_status(:created)
      expect(described_class.schema_errors('ApiAppTokenCreateResponse', response.parsed_body)).to be_empty

      get api_v1_household_admin_app_tokens_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('ApiAppTokenCollectionResponse', response.parsed_body)).to be_empty
      expect(response.parsed_body.fetch('data')).to all(satisfy { |token| token.exclude?('token') })
    end

    it 'rejects unsupported API app token request fields' do
      valid_request = { api_app_token: { name: 'Client token' } }
      invalid_request = valid_request.deep_merge(api_app_token: { token: 'private' })

      expect(described_class.schema_errors('ApiAppTokenCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('ApiAppTokenCreateRequest', invalid_request)).to include(
        '/api_app_token/token'
      )
    end

    it 'types the bounded household security audit event feed' do
      operation = described_class.operation('/households/{household_id}/admin/audit_logs', 'get')

      expect(operation.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/SecurityAuditEventCollectionResponse'
      )
      expect(described_class.schema('SecurityAuditEvent').fetch('additionalProperties')).to be(false)
      expect(described_class.schema('SecurityAuditEvent').dig('properties', 'metadata')).to eq(
        'type' => 'object', 'additionalProperties' => true
      )
      collection_schema = described_class.schema('SecurityAuditEventCollectionResponse')
      expect(collection_schema.dig('properties', 'data', 'maxItems')).to eq(100)
    end

    it 'matches the bounded descending Rails security audit event feed without leaking diagnostics' do
      household_id, headers = manager_api_context
      rows = 101.times.map { |index| contract_security_audit_event(household_id, index) }
      rows.each { |row| SecurityAuditEvent.create!(row) }

      get api_v1_household_admin_audit_logs_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      payload = response.parsed_body
      expect(described_class.schema_errors('SecurityAuditEventCollectionResponse', payload)).to be_empty
      expect(payload.fetch('data').size).to eq(100)
      expect(payload.fetch('data').pluck('created_at')).to eq(payload.fetch('data').pluck('created_at').sort.reverse)
      expect_private_audit_diagnostics(payload)
    end

    it 'types notification preference reads and updates' do
      path = '/households/{household_id}/notification_preference'
      get_operation = described_class.operation(path, 'get')
      patch_operation = described_class.operation(path, 'patch')
      put_operation = described_class.operation(path, 'put')

      expect(get_operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/NotificationPreferenceResponse'
      )
      [patch_operation, put_operation].each do |operation|
        expect(operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
          '#/components/schemas/NotificationPreferenceUpdateRequest'
        )
        expect(operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
          '#/components/schemas/NotificationPreferenceResponse'
        )
      end
    end

    it 'matches the notification preference Rails payload and rejects request drift' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))
      create(:notification_preference, person: users(:admin).person)

      get api_v1_household_notification_preference_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('NotificationPreferenceResponse', response.parsed_body)).to be_empty
      expect(
        described_class.schema_errors(
          'NotificationPreferenceUpdateRequest',
          notification_preference: { enabled: true, unexpected: true }
        )
      ).to include('/notification_preference/unexpected')
    end

    it 'rejects empty notification preference updates' do
      expect(
        described_class.schema_errors('NotificationPreferenceUpdateRequest', notification_preference: {})
      ).to include('/notification_preference')
    end

    it 'types native device token registration and revocation' do
      create_operation = described_class.operation('/households/{household_id}/native_device_tokens', 'post')
      delete_operation = described_class.operation('/households/{household_id}/native_device_tokens/{id}', 'delete')

      expect(create_operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/NativeDeviceTokenCreateRequest'
      )
      expect(create_operation.fetch('responses').keys).to include('201', '401', '403', '422')
      expect(delete_operation.fetch('responses').keys).to include('204', '401', '403')
    end

    it 'accepts only the native device token fields supported by Rails' do
      valid_request = {
        native_device_token: { device_token: 'opaque-device-token', platform: 'ios' }
      }
      invalid_request = {
        native_device_token: { device_token: 'opaque-device-token', platform: 'windows', secret: 'private' }
      }

      expect(described_class.schema_errors('NativeDeviceTokenCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('NativeDeviceTokenCreateRequest', invalid_request)).to contain_exactly(
        '/native_device_token/platform', '/native_device_token/secret'
      )
    end

    it 'types web push subscription registration, revocation, and testing' do
      path = '/households/{household_id}/push_subscription'
      create_operation = described_class.operation(path, 'post')
      delete_operation = described_class.operation(path, 'delete')
      test_operation = described_class.operation("#{path}/test", 'post')

      expect(create_operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PushSubscriptionCreateRequest'
      )
      expect(delete_operation.fetch('parameters')).to include(
        { '$ref' => '#/components/parameters/push_subscription_endpoint' }
      )
      expect(delete_operation.dig('responses', '400', '$ref')).to eq('#/components/responses/BadRequest')
      expect(test_operation.dig('responses', '503', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PushTestFailedErrorEnvelope'
      )
    end

    it 'requires the web push endpoint and keys without allowing secret response fields' do
      valid_request = {
        push_subscription: {
          endpoint: 'https://fcm.googleapis.com/fcm/send/subscription',
          keys: { p256dh: 'public-key', auth: 'auth-secret' }
        }
      }
      invalid_request = valid_request.deep_merge(push_subscription: { keys: { unexpected: 'secret' } })
      failed_response = {
        error: { code: 'push_test_failed', message: 'Unable to send test notification.', request_id: 'request-id' }
      }

      expect(described_class.schema_errors('PushSubscriptionCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('PushSubscriptionCreateRequest', invalid_request)).to include(
        '/push_subscription/keys/unexpected'
      )
      expect(described_class.schema_errors('PushTestFailedErrorEnvelope', failed_response)).to be_empty
    end

    it 'types medication lookup filters and outcomes' do
      operation = described_class.operation('/households/{household_id}/medication_lookup', 'get')
      query_parameters = operation.fetch('parameters').filter_map do |parameter|
        parameter['name'] if parameter['in'] == 'query'
      end

      expect(query_parameters).to contain_exactly('q', 'form', 'strength')
      expect(operation.fetch('responses').keys).to include('200', '401', '403', '404', '429', '503')
      expect(operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/MedicationLookupResponse'
      )
      expect(operation.dig('responses', '503', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/MedicationLookupUnavailableResponse'
      )

      unavailable_results = described_class.schema('MedicationLookupUnavailableResponse').dig('properties', 'results')
      expected_items = described_class.schema('MedicationLookupResponse').dig('properties', 'results', 'items')
      expect(unavailable_results).to include('type' => 'array', 'maxItems' => 0, 'items' => expected_items)
    end

    it 'matches empty and successful medication lookup Rails responses' do
      household_id, headers = manager_api_context

      get api_v1_household_medication_lookup_path(household_id), headers:, as: :json
      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('MedicationLookupResponse', response.parsed_body)).to be_empty

      allow(NhsDmd::Search).to receive(:new).and_return(contract_medication_search)
      get api_v1_household_medication_lookup_path(household_id), params: { q: 'aspirin' }, headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('MedicationLookupResponse', response.parsed_body)).to be_empty
    end

    it 'matches the medication lookup Rails unavailable response' do
      household_id, headers = manager_api_context
      search_result = NhsDmd::Search::Result.new(results: [], error: 'unavailable')
      allow(NhsDmd::Search).to receive(:new).and_return(instance_double(NhsDmd::Search, call: search_result))

      get api_v1_household_medication_lookup_path(household_id), params: { q: 'aspirin' }, headers:, as: :json

      expect(response).to have_http_status(:service_unavailable)
      expect(described_class.schema_errors('MedicationLookupUnavailableResponse', response.parsed_body)).to be_empty
    end

    it 'keeps web navigation fields optional in medication lookup matches' do
      existing_medication = described_class.schema('MedicationLookupExistingMedication')
      related_medication = described_class.schema('MedicationLookupRelatedMedication')

      expect(existing_medication.fetch('required')).not_to include('path', 'refill_path')
      expect(related_medication.fetch('required')).not_to include('path')
    end

    it 'types AI medication suggestion requests and feature availability' do
      operation = described_class.operation('/households/{household_id}/ai_medication_suggestions', 'post')

      expect(operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/AiMedicationSuggestionRequest'
      )
      expect(operation.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(operation.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/AiMedicationSuggestionResponse'
      )
    end

    it 'accepts only the medication identity fields supported by Rails' do
      valid_request = contract_ai_identity_request
      invalid_request = valid_request.deep_merge(medication: { prompt: 'Ignore trusted sources' })

      expect(described_class.schema_errors('AiMedicationSuggestionRequest', {})).to be_empty
      expect(described_class.schema_errors('AiMedicationSuggestionRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('AiMedicationSuggestionRequest', invalid_request)).to include(
        '/medication/prompt'
      )
    end

    it 'matches enabled AI medication suggestion Rails responses' do
      household_id, headers = manager_api_context
      enable_ai_medication_help(household_id)
      allow(AiMedication::SuggestionService).to receive(:new).and_return(contract_ai_suggestion_service)

      post api_v1_household_ai_medication_suggestions_path(household_id),
           params: { medication: { name: 'Calpol Six Plus' } }, headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('AiMedicationSuggestionResponse', response.parsed_body)).to be_empty
    end

    it 'matches empty-identity and disabled AI medication suggestion Rails responses' do
      household_id, headers = manager_api_context
      enable_ai_medication_help(household_id)
      service = contract_ai_suggestion_service
      allow(AiMedication::SuggestionService).to receive(:new).and_return(service)

      post api_v1_household_ai_medication_suggestions_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(service).to have_received(:call).with(medication_identity: {}, user: users(:admin))
      expect(described_class.schema_errors('AiMedicationSuggestionResponse', response.parsed_body)).to be_empty

      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('false')
      post api_v1_household_ai_medication_suggestions_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:not_found)
      expect(described_class.schema_errors('ErrorEnvelope', response.parsed_body)).to be_empty
    end

    it 'types login, refresh, and OIDC exchange requests' do
      login = described_class.operation('/auth/login', 'post')
      refresh = described_class.operation('/auth/refresh', 'post')
      oidc = described_class.operation('/auth/oidc_exchange', 'post')

      expect(auth_request_schema(login)).to eq('#/components/schemas/AuthLoginRequest')
      expect(auth_request_schema(refresh)).to eq('#/components/schemas/AuthRefreshRequest')
      expect(auth_request_schema(oidc)).to eq('#/components/schemas/AuthOidcExchangeRequest')
    end

    it 'types login, refresh, and OIDC exchange responses' do
      login = described_class.operation('/auth/login', 'post')
      refresh = described_class.operation('/auth/refresh', 'post')
      oidc = described_class.operation('/auth/oidc_exchange', 'post')

      expect(auth_response_schema(login, '201')).to eq('#/components/schemas/AuthLoginResponse')
      expect(auth_response_schema(refresh, '200')).to eq('#/components/schemas/AuthRefreshResponse')
      expect(auth_response_schema(oidc, '201')).to eq('#/components/schemas/AuthLoginResponse')
      expect([login, refresh, oidc]).to all(satisfy { |operation| operation.fetch('responses').key?('429') })
    end

    it 'accepts only the supported authentication request fields' do
      login = { email: 'admin@example.com', password: 'password', device_name: 'RSpec iPhone', household_id: 1 }
      refresh = { refresh_token: 'mt_refresh_token' }
      oidc = {
        id_token: 'signed-id-token', nonce: 'nonce', code_verifier: 'pkce-verifier',
        device_name: 'RSpec iPhone', household_id: 1, provider: 'oidc'
      }

      expect(described_class.schema_errors('AuthLoginRequest', login)).to be_empty
      expect(described_class.schema_errors('AuthRefreshRequest', refresh)).to be_empty
      expect(described_class.schema_errors('AuthOidcExchangeRequest', oidc)).to be_empty
      expect(described_class.schema_errors('AuthLoginRequest', login.merge(token: 'private'))).to include('/token')
    end

    it 'types authentication management and the current household profile' do
      households = described_class.operation('/auth/households', 'get')
      sessions = described_class.operation('/auth/sessions', 'get')
      revoke = described_class.operation('/auth/sessions/{id}', 'delete')
      profile = described_class.operation('/households/{household_id}/me', 'get')

      expect(auth_response_schema(households, '200')).to eq('#/components/schemas/AuthHouseholdCollectionResponse')
      expect(auth_response_schema(sessions, '200')).to eq('#/components/schemas/AuthSessionCollectionResponse')
      expect(revoke.fetch('responses').keys).to include('204', '401', '404', '429')
      expect(auth_response_schema(profile, '200')).to eq('#/components/schemas/MeResponse')
      expect(profile.fetch('responses').keys).to include('200', '401', '403', '404', '429')
    end

    it 'matches Rails login and refresh responses' do
      login_data = api_login(users(:admin))

      expect(response).to have_http_status(:created)
      expect(described_class.schema_errors('AuthLoginResponse', response.parsed_body)).to be_empty

      post api_v1_auth_refresh_path, params: { refresh_token: login_data.fetch('refresh_token') }, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('AuthRefreshResponse', response.parsed_body)).to be_empty
    end

    it 'matches Rails authentication collections and the current profile' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_auth_households_path, headers:, as: :json
      expect(described_class.schema_errors('AuthHouseholdCollectionResponse', response.parsed_body)).to be_empty

      get api_v1_auth_sessions_path, headers:, as: :json
      expect(described_class.schema_errors('AuthSessionCollectionResponse', response.parsed_body)).to be_empty

      get api_v1_household_me_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('MeResponse', response.parsed_body)).to be_empty
    end

    it 'types location reads and their household errors' do
      collection = described_class.operation('/households/{household_id}/locations', 'get')
      resource = described_class.operation('/households/{household_id}/locations/{id}', 'get')

      expect(auth_response_schema(collection, '200')).to eq('#/components/schemas/LocationCollectionResponse')
      expect(auth_response_schema(resource, '200')).to eq('#/components/schemas/LocationResponse')
      expect(collection.fetch('responses').keys).to include('200', '401', '403', '404', '422', '429')
      expect(resource.fetch('responses').keys).to include('200', '401', '403', '404', '429')
    end

    it 'types medication collections, resources, and writes' do
      collection_path = '/households/{household_id}/medications'
      resource_path = "#{collection_path}/{id}"
      collection = described_class.operation(collection_path, 'get')
      create = described_class.operation(collection_path, 'post')
      resource = described_class.operation(resource_path, 'get')
      updates = %w[patch put].map { |method| described_class.operation(resource_path, method) }

      expect(auth_response_schema(collection, '200')).to eq('#/components/schemas/MedicationCollectionResponse')
      expect(auth_response_schema(resource, '200')).to eq('#/components/schemas/MedicationResponse')
      expect(auth_request_schema(create)).to eq('#/components/schemas/MedicationCreateRequest')
      expect(updates).to all(satisfy do |operation|
        auth_request_schema(operation) == '#/components/schemas/MedicationUpdateRequest'
      end)
      expect([create, *updates]).to all(satisfy { |operation| operation.fetch('responses').key?('422') })
    end

    it 'types medication inventory and reorder actions' do
      base_path = '/households/{household_id}/medications/{id}'
      adjustment = described_class.operation("#{base_path}/adjust_inventory", 'patch')
      ordered = described_class.operation("#{base_path}/mark_as_ordered", 'patch')
      received = described_class.operation("#{base_path}/mark_as_received", 'patch')

      expect(auth_request_schema(adjustment)).to eq('#/components/schemas/MedicationInventoryAdjustmentRequest')
      expect(auth_request_schema(ordered)).to eq('#/components/schemas/MedicationOrderDetailsRequest')
      expect(ordered.dig('requestBody', 'required')).to be(false)
      expect(auth_response_schema(received, '200')).to eq('#/components/schemas/MedicationResponse')
      expect([adjustment, ordered, received]).to all(satisfy { |operation| operation.fetch('responses').key?('429') })
    end

    it 'accepts only supported medication write fields' do
      valid_create = {
        medication: { name: 'Contract medicine', location_id: 1, current_supply: '20', reorder_threshold: '5' }
      }
      invalid_create = valid_create.deep_merge(medication: { household_id: 99 })
      adjustment = { adjustment: { new_quantity: '10', reason: 'Stock count' } }

      expect(described_class.schema_errors('MedicationCreateRequest', valid_create)).to be_empty
      expect(described_class.schema_errors('MedicationCreateRequest', invalid_create)).to include(
        '/medication/household_id'
      )
      expect(described_class.schema_errors('MedicationInventoryAdjustmentRequest', adjustment)).to be_empty
    end

    it 'matches Rails location and medication collections' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_locations_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('LocationCollectionResponse', response.parsed_body)).to be_empty

      get api_v1_household_medications_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('MedicationCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'types dosage option collections, resources, and writes' do
      collection_path = '/households/{household_id}/dosage_options'
      resource_path = "#{collection_path}/{id}"
      collection = described_class.operation(collection_path, 'get')
      create = described_class.operation(collection_path, 'post')
      resource = described_class.operation(resource_path, 'get')
      updates = %w[patch put].map { |method| described_class.operation(resource_path, method) }

      expect(auth_response_schema(collection, '200')).to eq('#/components/schemas/DosageOptionCollectionResponse')
      expect(auth_response_schema(resource, '200')).to eq('#/components/schemas/DosageOptionResponse')
      expect(auth_request_schema(create)).to eq('#/components/schemas/DosageOptionCreateRequest')
      expect(updates).to all(satisfy do |operation|
        auth_request_schema(operation) == '#/components/schemas/DosageOptionUpdateRequest'
      end)
      expect([create, *updates]).to all(satisfy { |operation| operation.fetch('responses').key?('422') })
    end

    it 'accepts only supported dosage option write fields' do
      valid_create = {
        dosage_option: {
          medication_id: SecureRandom.uuid, amount: '500', unit: 'mg', frequency: 'Twice daily',
          default_max_daily_doses: 2, default_min_hours_between_doses: '6', default_dose_cycle: 'daily'
        }
      }
      invalid_create = valid_create.deep_merge(dosage_option: { household_id: 99 })

      expect(described_class.schema_errors('DosageOptionCreateRequest', valid_create)).to be_empty
      expect(described_class.schema_errors('DosageOptionCreateRequest', invalid_create)).to include(
        '/dosage_option/household_id'
      )
    end

    it 'matches the Rails dosage option collection' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_dosage_options_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('DosageOptionCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'types schedule and person medication operations' do
      schedule_path = '/households/{household_id}/schedules'
      person_medication_path = '/households/{household_id}/person_medications'

      expect(auth_response_schema(described_class.operation(schedule_path, 'get'), '200')).to eq(
        '#/components/schemas/ScheduleCollectionResponse'
      )
      expect(auth_request_schema(described_class.operation(schedule_path, 'post'))).to eq(
        '#/components/schemas/ScheduleCreateRequest'
      )
      expect(auth_response_schema(described_class.operation(person_medication_path, 'get'), '200')).to eq(
        '#/components/schemas/PersonMedicationCollectionResponse'
      )
      expect(auth_request_schema(described_class.operation(person_medication_path, 'post'))).to eq(
        '#/components/schemas/PersonMedicationCreateRequest'
      )
    end

    it 'types medication take operations and idempotent success' do
      path = '/households/{household_id}/medication_takes'
      collection = described_class.operation(path, 'get')
      create = described_class.operation(path, 'post')

      expect(auth_response_schema(collection, '200')).to eq('#/components/schemas/MedicationTakeCollectionResponse')
      expect(auth_request_schema(create)).to eq('#/components/schemas/MedicationTakeCreateRequest')
      expect(auth_response_schema(create, '201')).to eq('#/components/schemas/MedicationTakeResponse')
      expect(auth_response_schema(create, '200')).to eq('#/components/schemas/MedicationTakeResponse')
    end

    it 'rejects unsupported schedule fields' do
      schedule = {
        schedule: {
          person_id: SecureRandom.uuid, medication_id: '1', dose_amount: '5', dose_unit: 'ml',
          start_date: Date.current.iso8601, end_date: 1.month.from_now.to_date.iso8601
        }
      }

      invalid_schedule = schedule.deep_merge(schedule: { secret: true })

      expect(described_class.schema_errors('ScheduleCreateRequest', schedule)).to be_empty
      expect(described_class.schema_errors('ScheduleCreateRequest', invalid_schedule)).to include('/schedule/secret')
    end

    it 'rejects unsupported person medication fields' do
      person_medication = {
        person_medication: { person_id: '1', medication_id: SecureRandom.uuid, administration_kind: 'routine' }
      }

      expect(described_class.schema_errors('PersonMedicationCreateRequest', person_medication)).to be_empty
      expect(
        described_class.schema_errors(
          'PersonMedicationCreateRequest', person_medication.deep_merge(person_medication: { household_id: 9 })
        )
      ).to include('/person_medication/household_id')
    end

    it 'matches Rails medication administration collections' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_schedules_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('ScheduleCollectionResponse', response.parsed_body)).to be_empty

      get api_v1_household_person_medications_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('PersonMedicationCollectionResponse', response.parsed_body)).to be_empty

      get api_v1_household_medication_takes_path(household_id), headers:, as: :json
      expect(described_class.schema_errors('MedicationTakeCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'types health event collections, resources, and writes' do
      collection_path = '/households/{household_id}/health_events'
      resource_path = "#{collection_path}/{id}"
      create = described_class.operation(collection_path, 'post')
      updates = %w[patch put].map { |method| described_class.operation(resource_path, method) }

      expect(auth_response_schema(described_class.operation(collection_path, 'get'), '200')).to eq(
        '#/components/schemas/HealthEventCollectionResponse'
      )
      expect(auth_response_schema(described_class.operation(resource_path, 'get'), '200')).to eq(
        '#/components/schemas/HealthEventResponse'
      )
      expect(auth_request_schema(create)).to eq('#/components/schemas/HealthEventCreateRequest')
      expect(updates).to all(satisfy do |operation|
        auth_request_schema(operation) == '#/components/schemas/HealthEventUpdateRequest'
      end)
    end

    it 'rejects unsupported health event fields' do
      valid_request = {
        health_event: {
          person_id: SecureRandom.uuid, event_kind: 'illness', severity: 'moderate',
          title: 'Seasonal cold', started_on: Date.current.iso8601, medication_ids: ['1']
        }
      }
      invalid_request = valid_request.deep_merge(health_event: { household_id: 9 })

      expect(described_class.schema_errors('HealthEventCreateRequest', valid_request)).to be_empty
      expect(described_class.schema_errors('HealthEventCreateRequest', invalid_request)).to include(
        '/health_event/household_id'
      )
    end

    it 'matches the Rails health event collection' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_health_events_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('HealthEventCollectionResponse', response.parsed_body)).to be_empty
    end

    it 'types sync changes and batch mutations' do
      changes = described_class.operation('/households/{household_id}/sync/changes', 'get')
      batch = described_class.operation('/households/{household_id}/sync/batches', 'post')

      expect(auth_response_schema(changes, '200')).to eq('#/components/schemas/SyncChangesResponse')
      expect(auth_request_schema(batch)).to eq('#/components/schemas/SyncBatchRequest')
      expect(auth_response_schema(batch, '201')).to eq('#/components/schemas/SyncBatchResponse')
      expect(batch.fetch('responses').keys).to include('201', '400', '401', '403', '404', '409', '422', '428', '429')
    end

    it 'matches the Rails sync change feed' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_sync_changes_path(household_id),
          params: { cursor: 1.minute.from_now.iso8601 }, headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('SyncChangesResponse', response.parsed_body)).to be_empty
    end

    it 'accepts medication take creation in a sync batch' do
      request = {
        batch: {
          operations: [
            {
              action: 'create', resource_type: 'medication_take',
              attributes: { client_uuid: SecureRandom.uuid, source_type: 'schedule', source_id: SecureRandom.uuid }
            }
          ]
        }
      }

      expect(described_class.schema_errors('SyncBatchRequest', request)).to be_empty
    end

    it 'types encrypted portable exports and imports' do
      export = described_class.operation('/households/{household_id}/portable_export', 'get')
      dry_run = described_class.operation('/households/{household_id}/portable_imports/dry_run', 'post')
      import = described_class.operation('/households/{household_id}/portable_imports', 'post')

      expect(auth_response_schema(export, '200')).to eq('#/components/schemas/PortableEnvelopeResponse')
      expect(auth_request_schema(dry_run)).to eq('#/components/schemas/PortableImportRequest')
      expect(auth_response_schema(dry_run, '200')).to eq('#/components/schemas/PortableImportResultResponse')
      expect(auth_request_schema(import)).to eq('#/components/schemas/PortableImportRequest')
      expect(auth_response_schema(import, '201')).to eq('#/components/schemas/PortableImportResultResponse')
    end

    it 'models the portable import 422 response' do
      import = described_class.operation('/households/{household_id}/portable_imports', 'post')

      expect(auth_response_schema(import, '422')).to eq('#/components/schemas/PortableImportUnprocessableResponse')
    end

    it 'models both portable import 422 envelope shapes without a union' do
      result_envelope = {
        data: { applied: false, counts: {}, conflicts: [], errors: ['Import was not applied'] }
      }
      error_envelope = {
        error: { code: 'validation_failed', message: 'Validation failed', request_id: SecureRandom.uuid }
      }

      expect(described_class.schema_errors('PortableImportUnprocessableResponse', result_envelope)).to be_empty
      expect(described_class.schema_errors('PortableImportUnprocessableResponse', error_envelope)).to be_empty
      expect(
        described_class.schema_errors(
          'PortableImportUnprocessableResponse',
          result_envelope.merge(error_envelope)
        )
      ).not_to be_empty
    end

    it 'accepts only the encrypted portable envelope fields' do
      envelope = {
        bundle: {
          format: 'medtracker.portable.encrypted.v1', encrypted_at: Time.current.iso8601,
          cipher: 'aes-256-gcm', kdf: 'pbkdf2_sha256', salt: 'salt', checksum: 'a' * 64,
          ciphertext: 'secret'
        }
      }

      expect(described_class.schema_errors('PortableImportRequest', envelope)).to be_empty
      expect(
        described_class.schema_errors('PortableImportRequest', envelope.deep_merge(bundle: { passphrase: 'secret' }))
      ).to include('/bundle/passphrase')
    end

    it 'types mobile and consistent sync snapshots' do
      mobile = described_class.operation('/households/{household_id}/mobile_snapshot', 'get')
      sync = described_class.operation('/households/{household_id}/sync/snapshot', 'get')

      expect(auth_response_schema(mobile, '200')).to eq('#/components/schemas/PortableSnapshotResponse')
      expect(auth_response_schema(sync, '200')).to eq('#/components/schemas/SyncSnapshotResponse')
      expect(mobile.fetch('responses').keys).to include('200', '401', '403', '404', '429')
      expect(sync.fetch('responses').keys).to include('200', '401', '403', '404', '429')
    end

    it 'types the profile export modes and response shapes' do
      export = described_class.operation('/households/{household_id}/data_exports/{mode}', 'get')
      mode_schema = export.fetch('parameters').find { |parameter| parameter['name'] == 'mode' }.fetch('schema')
      response_schema = export.dig('responses', '200', 'content', 'application/json', 'schema')

      expect(mode_schema.fetch('enum')).to contain_exactly(
        'encrypted_migration_bundle', 'backup_zip', 'health_data_json'
      )
      expect(response_schema.fetch('$ref')).to eq('#/components/schemas/DataExportResponse')
      expect(export.dig('responses', '200', 'headers', 'Cache-Control', 'schema')).to include(
        'type' => 'string', 'enum' => ['no-store']
      )
    end

    it 'matches the Rails mobile snapshot' do
      login_data = api_login(users(:admin))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_mobile_snapshot_path(household_id), headers:, as: :json

      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('PortableSnapshotResponse', response.parsed_body)).to be_empty
    end

    it 'references typed representative person request and response schemas' do
      create_person = described_class.operation('/households/{household_id}/people', 'post')
      show_person = described_class.operation('/households/{household_id}/people/{id}', 'get')

      expect(create_person.dig('requestBody', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonCreateRequest'
      )
      expect(create_person.dig('responses', '201', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonResponse'
      )
      expect(show_person.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/PersonResponse'
      )
    end

    it 'does not advertise unsupported person update preconditions' do
      update_person = described_class.operation('/households/{household_id}/people/{id}', 'patch')

      expect(update_person.fetch('parameters')).not_to include(
        { '$ref' => '#/components/parameters/if_match' }
      )
      expect(update_person.fetch('responses')).not_to include('409')
      expect(update_person.dig('responses', '422', '$ref')).to eq('#/components/responses/ValidationFailed')
    end

    it 'types the arbitrary native device token path separately' do
      native_device_token = described_class.operation(
        '/households/{household_id}/native_device_tokens/{id}', 'delete'
      )
      expect(native_device_token.fetch('parameters')).to include(
        { '$ref' => '#/components/parameters/native_device_token' }
      )
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

    def manager_api_context
      login_data = api_login(users(:admin))
      session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))

      [login_data.dig('household', 'id'), api_auth_headers(login_data.fetch('access_token')), session]
    end

    def contract_membership(household_id)
      account = Account.create!(email: "contract-member-#{SecureRandom.hex(4)}@example.test", status: :verified)
      Household.find(household_id).household_memberships.create!(account:, role: :member)
    end

    def contract_grant_attributes(membership)
      {
        household_membership_id: membership.id,
        person_id: people(:john).id,
        access_level: 'manage',
        relationship_type: 'carer'
      }
    end

    def contract_security_audit_event(household_id, index)
      timestamp = 2.hours.from_now + index.seconds
      {
        household_id:,
        event_type: "contract.audit.#{index}",
        metadata: { 'sequence' => index },
        audit_context: {},
        request_id: "contract-request-#{index}",
        created_at: timestamp,
        updated_at: timestamp
      }
    end

    def contract_medication_search
      search_result = NhsDmd::SearchResult.new(
        code: '39720311000001101',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        concept_class: 'VMP',
        package_quantity: 28,
        package_unit: 'tablet',
        directions: 'Take with water',
        warnings: 'Follow the medicine label.'
      )
      outcome = NhsDmd::Search::Result.new(results: [search_result], error: nil, resolved_query: 'aspirin')

      instance_double(NhsDmd::Search, call: outcome)
    end

    def contract_ai_suggestion_service
      suggestion = AiMedication::Suggestion.new(
        medication: { name: 'Calpol Six Plus', description: 'Paracetamol pain and fever relief' },
        doses: [contract_ai_dose],
        sources: [{ url: contract_ai_source_url, title: 'CALPOL SixPlus' }]
      )

      instance_double(AiMedication::SuggestionService, call: suggestion)
    end

    def contract_ai_identity_request
      {
        medication: {
          name: 'Calpol Six Plus', barcode: '5010123730215', dmd_code: '123456789',
          dmd_system: 'https://dmd.nhs.uk', dmd_concept_class: 'AMP', category: 'Pain relief',
          description: 'Paracetamol oral suspension'
        }
      }
    end

    def contract_ai_dose
      {
        amount: '5',
        unit: 'ml',
        description: 'Children 6-8 years',
        default_max_daily_doses: 4,
        default_min_hours_between_doses: '4',
        default_dose_cycle: 'daily',
        evidence: {
          url: contract_ai_source_url,
          title: 'CALPOL SixPlus',
          text: 'Children 6-8 years 5ml up to 4 times in 24 hours.'
        }
      }
    end

    def contract_ai_source_url
      'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol'
    end

    def enable_ai_medication_help(household_id)
      Household.find(household_id).update!(subscription_plan: 'family_plus')
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')
    end

    def auth_request_schema(operation)
      operation.dig('requestBody', 'content', 'application/json', 'schema', '$ref')
    end

    def auth_response_schema(operation, status)
      operation.dig('responses', status, 'content', 'application/json', 'schema', '$ref')
    end

    def expect_private_audit_diagnostics(payload)
      invalid_payload = payload.deep_dup
      invalid_payload.dig('data', 0)['id'] = 'private-audit-payload-value'
      diagnostics = described_class.schema_errors('SecurityAuditEventCollectionResponse', invalid_payload).join(' ')
      expect(diagnostics).to include('/data/0/id')
      expect(diagnostics).not_to include('private-audit-payload-value')
    end
  end
end
