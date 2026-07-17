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
    'spec/fixtures/files/openapi-3.1-schema-2022-10-07.json'
  )

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
        'POST /auth/select_household',
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

    it 'declares an OpenAPI 3.1 document with required sections' do
      expect(described_class.document).to include('openapi' => '3.1.0')
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

    it 'fully types dashboard and medication take history operations' do
      dashboard = described_class.operation('/households/{household_id}/dashboard', 'get')
      history = described_class.operation('/households/{household_id}/medication_takes', 'get')

      dashboard_parameters = dashboard.fetch('parameters').filter_map { |parameter| parameter['name'] }
      history_parameters = history.fetch('parameters').filter_map { |parameter| parameter['name'] }

      expect(dashboard_parameters).to contain_exactly('date', 'person_id')
      expect(dashboard.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/DashboardResponse'
      )
      expect(history_parameters).to contain_exactly(
        'pagination', 'cursor', 'person_id', 'from', 'to', 'per_page', 'page', 'updated_since'
      )
      expect(history.dig('responses', '200', 'content', 'application/json', 'schema', '$ref')).to eq(
        '#/components/schemas/MedicationTakeCollectionResponse'
      )
    end

    it 'fully types dashboard task and response schemas' do
      task_schema = described_class.schema('DashboardTask')
      expect(task_schema.dig('properties', 'status', 'enum')).to contain_exactly(
        'due', 'upcoming', 'available', 'cooldown', 'max_reached', 'paused', 'out_of_stock',
        'selection_required'
      )
      expect(described_class.schema('DashboardResponse').dig('properties', 'data', '$ref')).to eq(
        '#/components/schemas/Dashboard'
      )
    end

    it 'fully types medication take history response schemas' do
      expect(described_class.schema('MedicationTakeCursorMeta').fetch('required')).to contain_exactly(
        'per_page', 'next_cursor', 'has_more'
      )
      expect(described_class.schema('MedicationTake').dig('properties', 'reversal', 'type')).to contain_exactly(
        'object', 'null'
      )
    end

    it 'validates representative dashboard and cursor history responses against their schemas' do
      login_data = api_login(users(:jane))
      household_id = login_data.dig('household', 'id')
      headers = api_auth_headers(login_data.fetch('access_token'))

      get api_v1_household_dashboard_path(household_id), params: { person_id: 'all' }, headers:, as: :json
      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('DashboardResponse', response.parsed_body)).to be_empty

      get api_v1_household_medication_takes_path(household_id),
          params: { pagination: 'cursor' }, headers:, as: :json
      expect(response).to have_http_status(:ok)
      expect(described_class.schema_errors('MedicationTakeCollectionResponse', response.parsed_body)).to be_empty
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
  end
end
