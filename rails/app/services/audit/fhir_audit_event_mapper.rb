# frozen_string_literal: true

require 'cgi'

module Audit
  class FhirAuditEventMapper
    TYPE_SYSTEM = 'http://dicom.nema.org/resources/ontology/DCM'
    SUBTYPE_SYSTEM = 'https://medtracker.app/fhir/CodeSystem/audit-event-type'
    ROLE_SYSTEM = 'https://medtracker.app/fhir/CodeSystem/household-role'

    def initialize(entry)
      @envelope = entry.envelope
    end

    def to_h
      {
        resourceType: 'AuditEvent', id: envelope['event_id'],
        type: { system: TYPE_SYSTEM, code: '110100', display: 'Application Activity' },
        subtype: [{ system: SUBTYPE_SYSTEM, code: envelope['event_type'] }],
        action: action, recorded: envelope['occurred_at'], outcome: outcome,
        agent: [agent], source:, entity: [entity]
      }.compact
    end

    private

    attr_reader :envelope

    def action
      event_type = envelope['event_type'].to_s
      return 'C' if event_type.match?(/create|record|login|start/)
      return 'R' if event_type.match?(/read|view|access|export/)
      return 'U' if event_type.match?(/update|change|rotate|restock|take/)
      return 'D' if event_type.match?(/delete|destroy|revoke|logout/)

      'E'
    end

    def outcome
      envelope['outcome'].to_s == 'success' ? '0' : '8'
    end

    def agent
      agent = envelope.fetch('agent', {})
      {
        who: { identifier: { system: 'https://medtracker.app/identifier/account', value: agent['account_id'].to_s } },
        requestor: agent['account_id'].present?,
        role: agent_role(agent['role']),
        policy: policy_uris,
        network: network
      }.compact
    end

    def agent_role(role)
      return if role.blank?

      [{ coding: [{ system: ROLE_SYSTEM, code: role }] }]
    end

    def policy_uris
      policy = envelope.fetch('policy', {})
      return if policy.values.compact.empty?

      value = [policy['class'], policy['query']].compact.join('/')
      ["https://medtracker.app/policy/#{CGI.escape(value)}"]
    end

    def network
      address = envelope.dig('request', 'ip')
      { address:, type: '2' } if address.present?
    end

    def source
      {
        observer: {
          identifier: { system: 'https://medtracker.app/identifier/audit-source', value: 'medtracker' }
        },
        type: [{ system: 'http://terminology.hl7.org/CodeSystem/security-source-type', code: '4' }]
      }
    end

    def entity
      source = envelope.fetch('source', {})
      {
        what: {
          identifier: {
            system: 'https://medtracker.app/identifier/audit-source-row',
            value: [source['table'], source['id']].compact.join('/')
          }
        },
        name: envelope.dig('entity', 'type')
      }.compact
    end
  end
end
