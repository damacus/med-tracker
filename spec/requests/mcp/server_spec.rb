# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'MedTracker MCP server' do
  fixtures :accounts, :people, :users, :households, :locations, :medications, :schedules,
           :person_medications, :medication_takes

  before do
    grant_jane_manage_access!
  end

  it 'rejects unauthenticated MCP requests' do
    mcp_post('tools/list', headers: mcp_headers.except('Authorization'))

    expect(response).to have_http_status(:unauthorized)
    expect(response.parsed_body.dig('error', 'code')).to eq('unauthorized')
  end

  it 'throttles repeated MCP requests at the Rack boundary' do
    with_rack_attack_enabled do
      60.times { mcp_post('tools/list', headers: mcp_headers.except('Authorization')) }
      expect(response).to have_http_status(:unauthorized)

      mcp_post('tools/list', headers: mcp_headers.except('Authorization'))
    end

    expect(response).to have_http_status(:too_many_requests)
    expect(response.headers['Retry-After'].to_i).to be_positive
  end

  it 'initializes the server and publishes read-only tools, resources, and prompts' do
    mcp_post(
      'initialize',
      params: {
        protocolVersion: '2025-11-25',
        capabilities: {},
        clientInfo: { name: 'RSpec MCP client', version: '1.0' }
      }
    )

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('result', 'serverInfo')).to include(
      'name' => 'med_tracker',
      'title' => 'MedTracker MCP'
    )

    mcp_post('tools/list')
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('result', 'tools').pluck('name')).to include(
      'medtracker_current_user',
      'medtracker_household_snapshot',
      'medtracker_today_schedule',
      'medtracker_inventory_risks',
      'medtracker_health_history_summary'
    )

    mcp_post('resources/list')
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('result', 'resources')).to include(
      hash_including('uri' => 'medtracker://household/snapshot', 'name' => 'household_snapshot')
    )

    mcp_post('prompts/list')
    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('result', 'prompts')).to include(
      hash_including('name' => 'medtracker_household_review')
    )
  end

  it 'runs tools inside the authenticated tenant context and audits the MCP request' do
    mcp_post(
      'tools/call',
      params: {
        name: 'medtracker_current_user',
        arguments: {}
      }
    )

    expect(response).to have_http_status(:ok)
    result = response.parsed_body.fetch('result')
    expect(result.dig('structuredContent', 'user', 'email_address')).to eq(users(:jane).email_address)

    audit_event = SecurityAuditEvent.where(event_type: 'mcp.request').order(:created_at).last
    expect(audit_event).to have_attributes(
      household: households(:fixture_household),
      actor_account: accounts(:jane_doe),
      actor_membership: membership,
      request_id: be_present
    )
    expect(audit_event.metadata).to include('method' => 'tools/call', 'outcome' => 'ok')
    expect(audit_event.audit_context).to include(
      'authentication_method' => 'api_app_token',
      'session_reference' => "api_app_token:#{app_token.id}",
      'actor_account_id' => accounts(:jane_doe).id,
      'actor_membership_id' => membership.id,
      'active_role' => 'member',
      'permissions_version' => membership.permissions_version
    )
    expect(audit_event.metadata.to_json).not_to include(raw_token)
    expect(audit_event.audit_context.to_json).not_to include(raw_token)
  end

  it 'does not let audit write failures replace the transport response' do
    allow(SecurityAuditEvent).to receive(:create!).and_raise(ActiveRecord::ConnectionNotEstablished, 'audit offline')

    mcp_post('tools/list')

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('result', 'tools').pluck('name')).to include('medtracker_current_user')
  end

  it 'keeps batch JSON-RPC request auditing from replacing the transport response' do
    post '/mcp',
         params: [
           { jsonrpc: '2.0', id: next_request_id, method: 'tools/list', params: {} }
         ],
         headers: mcp_headers,
         as: :json

    expect(response).to have_http_status(:bad_request)
    expect(response.parsed_body.dig('error', 'message')).to include('JSON-RPC body must be a single request object')

    audit_event = SecurityAuditEvent.where(event_type: 'mcp.request').order(:created_at).last
    expect(audit_event.metadata).to include('status' => 400)
    expect(audit_event.metadata).not_to have_key('method')
  end

  it 'reads the household snapshot resource through the MCP resource API' do
    mcp_post(
      'resources/read',
      params: { uri: 'medtracker://household/snapshot' }
    )

    expect(response).to have_http_status(:ok)
    contents = response.parsed_body.dig('result', 'contents')
    expect(contents).to include(hash_including('uri' => 'medtracker://household/snapshot'))
    snapshot = JSON.parse(contents.first.fetch('text'))
    expect(snapshot).to include('format' => 'medtracker.mcp.household_snapshot.v1')
  end

  def mcp_post(method, params: {}, headers: mcp_headers)
    post '/mcp',
         params: { jsonrpc: '2.0', id: next_request_id, method: method, params: params },
         headers: headers,
         as: :json
  end

  def next_request_id
    @next_request_id ||= 0
    @next_request_id += 1
  end

  def mcp_headers
    {
      'Authorization' => "Bearer #{raw_token}",
      'Accept' => 'application/json'
    }
  end

  def with_rack_attack_enabled
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled

    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true

    yield
  ensure
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
  end

  def raw_token
    @raw_token ||= issued_app_token.last
  end

  def app_token
    issued_app_token.first
  end

  def issued_app_token
    @issued_app_token ||= ApiAppToken.issue_for(
      account: accounts(:jane_doe),
      household_membership: membership,
      name: 'RSpec MCP server token',
      audit_context: { request_id: 'mcp-server-token' }
    )
  end

  def membership
    @membership ||= households(:fixture_household).household_memberships.find_or_create_by!(
      account: accounts(:jane_doe)
    ) do |household_membership|
      household_membership.person = people(:jane)
      household_membership.role = :member
      household_membership.status = :active
    end
  end

  def grant_jane_manage_access!
    PersonAccessGrant.find_or_create_by!(
      household: households(:fixture_household),
      household_membership: membership,
      person: people(:jane)
    ) do |grant|
      grant.access_level = :manage
      grant.relationship_type = :self
      grant.granted_by_membership = membership
    end
  end
end
