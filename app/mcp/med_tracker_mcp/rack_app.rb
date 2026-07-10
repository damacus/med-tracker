# frozen_string_literal: true

require 'mcp'

module MedTrackerMcp
  class RackApp
    def call(env)
      request = ActionDispatch::Request.new(env)
      context = Context.resolve!(request)

      context.with_current do
        audit_request(request, context) do
          transport_for(context).call(env)
        end
      end
    rescue Context::AuthenticationError => e
      unauthorized_response(e)
    end

    private

    def transport_for(context)
      MCP::Server::Transports::StreamableHTTPTransport.new(
        Server.build(server_context: context.to_h),
        stateless: true,
        enable_json_response: true
      )
    end

    def audit_request(request, context)
      response = nil
      outcome = 'ok'

      response = yield
    rescue StandardError
      outcome = 'error'
      raise
    ensure
      record_audit_event(request, context, outcome, response)
    end

    def record_audit_event(request, context, outcome, response)
      context_data = context.to_h

      Audit::Event.record!(**audit_event_attributes(request, context_data, outcome, response))
    rescue StandardError => e
      Rails.logger.warn("MCP audit event failed: #{e.class}: #{e.message}")
    end

    def audit_event_attributes(request, context_data, outcome, response)
      {
        household: context_data.fetch(:household),
        actor_account: context_data.fetch(:account),
        actor_membership: context_data.fetch(:membership),
        event_type: 'mcp.request',
        request_id: context_data.fetch(:request_id),
        ip: context_data.fetch(:remote_ip),
        metadata: audit_event_metadata(request, outcome, response)
      }
    end

    def audit_event_metadata(request, outcome, response)
      {
        method: json_rpc_method(request),
        outcome: outcome,
        status: response&.first
      }.compact
    end

    def json_rpc_method(request)
      request.body.rewind
      parsed = JSON.parse(request.body.read)
      parsed.fetch('method', nil) if parsed.is_a?(Hash)
    rescue JSON::ParserError
      nil
    ensure
      request.body.rewind
    end

    def unauthorized_response(error)
      body = {
        error: {
          code: error.code,
          message: error.message
        }
      }

      [Rack::Utils.status_code(error.status), { 'Content-Type' => 'application/json' }, [JSON.generate(body)]]
    end
  end
end
