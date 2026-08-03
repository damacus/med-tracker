# frozen_string_literal: true

namespace :canary do
  desc 'Replace canary runtime state with the committed synthetic demo baseline'
  task demo_reset: :environment do
    result = DemoReset::Runner.new.call
    puts({
      event_type: 'canary.demo_reset',
      outcome: result.fetch(:outcome),
      baseline: result.fetch(:baseline)
    }.to_json)
  rescue StandardError => e
    reason = case e
             when DemoReset::UnsafeTargetError then 'unsafe_target'
             when DemoBaseline::Loader::InvalidBaselineError then 'invalid_baseline'
             when DemoReset::StorageCleanupError then 'storage_cleanup_failed'
             when DemoReset::VerificationError then 'verification_failed'
             else 'reset_failed'
             end
    abort({ event_type: 'canary.demo_reset', outcome: 'failed', reason: }.to_json)
  end
end
