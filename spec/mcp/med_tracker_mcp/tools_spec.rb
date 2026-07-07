# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedTrackerMcp::Tools do
  fixtures :accounts, :people, :users, :households, :locations, :medications, :schedules,
           :person_medications, :medication_takes

  before do
    grant_jane_manage_access!
    medications(:ibuprofen).update!(current_supply: 1, reorder_threshold: 5, supply_at_last_restock: 30)
  end

  it 'returns the current API user profile' do
    payload = call_tool(MedTrackerMcp::Tools::CurrentUserTool)

    expect(payload.fetch(:format)).to eq('medtracker.mcp.current_user.v1')
    expect(payload.dig(:user, :email_address)).to eq(users(:jane).email_address)
    expect(payload.dig(:user, :person, :id)).to eq(people(:jane).id)
  end

  it 'returns a policy-scoped household snapshot' do
    payload = call_tool(MedTrackerMcp::Tools::HouseholdSnapshotTool)

    expect(payload.fetch(:format)).to eq('medtracker.mcp.household_snapshot.v1')
    people = payload.dig(:snapshot, :records, :people)
    expect(people.pluck(:portable_id)).to contain_exactly(
      people(:jane).portable_id,
      people(:child_patient).portable_id
    )
  end

  it 'returns today schedule context with taken medication summaries' do
    payload = call_tool(MedTrackerMcp::Tools::TodayScheduleTool)

    expect(payload.fetch(:format)).to eq('medtracker.mcp.today_schedule.v1')
    expect(payload.fetch(:schedules).pluck(:id)).to include(schedules(:jane_ibuprofen).id)
    expect(payload.fetch(:taken_today)).to include(
      hash_including(person_id: people(:jane).id, medications: [hash_including(name: 'Ibuprofen')])
    )
  end

  it 'returns inventory risks from visible medications only' do
    payload = call_tool(MedTrackerMcp::Tools::InventoryRisksTool)

    expect(payload.fetch(:format)).to eq('medtracker.mcp.inventory_risks.v1')
    expect(payload.fetch(:medications)).to include(hash_including(id: medications(:ibuprofen).id, low_stock: true))
    expect(payload.fetch(:medications).pluck(:id)).not_to include(medications(:paracetamol).id)
  end

  it 'returns a bounded health history summary for visible people' do
    payload = call_tool(
      MedTrackerMcp::Tools::HealthHistorySummaryTool,
      start_date: 10.days.ago.to_date.iso8601,
      end_date: Time.zone.today.iso8601
    )

    expect(payload.fetch(:format)).to eq('medtracker.mcp.health_history_summary.v1')
    expect(payload.fetch(:people).pluck(:id)).to contain_exactly(people(:jane).id, people(:child_patient).id)
    expect(payload.fetch(:medication_takes).pluck(:medication_name)).to include('Ibuprofen')
  end

  it 'describes the household snapshot resource' do
    resource = MedTrackerMcp::Resources::HouseholdSnapshot.definition

    expect(resource.to_h).to include(
      uri: 'medtracker://household/snapshot',
      name: 'household_snapshot',
      mimeType: 'application/json'
    )
  end

  def call_tool(tool, **arguments)
    context.with_current do
      tool.call(**arguments, server_context: context.to_h).to_h.fetch(:structuredContent)
    end
  end

  def context
    @context ||= MedTrackerMcp::Context.resolve!(request_for(raw_token))
  end

  def request_for(token)
    Struct.new(:headers, :request_id, :remote_ip, keyword_init: true).new(
      headers: { 'Authorization' => "Bearer #{token}" },
      request_id: 'tool-request',
      remote_ip: '203.0.113.20'
    )
  end

  def raw_token
    @raw_token ||= ApiAppToken.issue_for(
      account: accounts(:jane_doe),
      household_membership: membership,
      name: 'RSpec MCP tools token',
      audit_context: { request_id: 'tool-token' }
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
