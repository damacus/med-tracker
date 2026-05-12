# frozen_string_literal: true

class AuthTokenAuditLogger
  ITEM_TYPE = 'AuthenticationToken'
  HASHED_FIELDS = %i[endpoint user_agent].freeze

  def record(account:, token_type:, action:, metadata: {}, context: nil)
    # rubocop:disable Rails/SkipsModelValidations
    PaperTrail::Version.insert(version_attrs(account:, token_type:, action:, metadata:, context:))
    # rubocop:enable Rails/SkipsModelValidations
  rescue StandardError => e
    Rails.logger.error("AuthTokenAuditLogger failed: #{e.class}: #{e.message}")
  end

  private

  def version_attrs(account:, token_type:, action:, metadata:, context:)
    {
      item_type: ITEM_TYPE,
      item_id: account.id,
      event: "auth_token/#{token_type}/#{action}",
      object: version_object(account:, token_type:, action:, metadata:).to_json,
      whodunnit: context_value(context, :whodunnit)&.to_s,
      ip: context_value(context, :ip),
      request_id: context_value(context, :request_id),
      created_at: Time.current
    }
  end

  def version_object(account:, token_type:, action:, metadata:)
    {
      account_id: account.id,
      token_type: token_type.to_s,
      action: action.to_s
    }.merge(redacted_metadata(metadata))
  end

  def redacted_metadata(metadata)
    data = metadata.to_h.symbolize_keys
    hashed_metadata(data).merge(device_name_metadata(data))
                         .merge(optional_value(data, :platform))
                         .merge(optional_value(data, :credential_nickname))
                         .merge(expires_at_metadata(data))
  end

  def context_value(context, key)
    return context[key] if context&.key?(key)

    paper_trail_context[key]
  end

  def paper_trail_context
    {
      whodunnit: PaperTrail.request.whodunnit,
      ip: PaperTrail.request.controller_info&.dig(:ip),
      request_id: PaperTrail.request.controller_info&.dig(:request_id)
    }
  end

  def hashed_metadata(data)
    HASHED_FIELDS.each_with_object({}) do |field, redacted|
      redacted[:"#{field}_hash"] = Digest::SHA256.hexdigest(data[field].to_s) if data[field].present?
    end
  end

  def device_name_metadata(data)
    return {} unless data.key?(:device_name)

    device_name = data[:device_name].to_s
    metadata = { device_name_present: device_name.present? }
    metadata[:device_name_length] = device_name.length if device_name.present?
    metadata
  end

  def optional_value(data, key)
    return {} if data[key].blank?

    { key => data[key].to_s }
  end

  def expires_at_metadata(data)
    return {} if data[:expires_at].blank?

    value = data[:expires_at]
    { expires_at: value.respond_to?(:iso8601) ? value.iso8601 : value.to_s }
  end
end
