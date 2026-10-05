# frozen_string_literal: true

namespace :support_access do
  desc 'Record natural support-access expiry events exactly once'
  task expire: :environment do
    processed_count = ExpireSupportAccessSessionsJob.perform_now
    puts({
      event_type: SupportAccessSessions::ExpiryProcessor::EVENT_TYPE,
      outcome: 'success',
      processed_count: processed_count
    }.to_json)
  end
end
