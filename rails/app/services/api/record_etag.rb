# frozen_string_literal: true

module Api
  class RecordEtag
    def self.for(record)
      %("#{Digest::SHA256.hexdigest([record.class.name, record.id, record.updated_at.to_f].join(':'))}")
    end
  end
end
