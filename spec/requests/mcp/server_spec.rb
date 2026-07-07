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
    expect(audit_event.metadata.to_json).not_to include(raw_token)
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

  def raw_token
    @raw_token ||= ApiAppToken.issue_for(
      account: accounts(:jane_doe),
      household_membership: membership,
      name: 'RSpec MCP server token',
      audit_context: { request_id: 'mcp-server-token' }
    ).last
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
