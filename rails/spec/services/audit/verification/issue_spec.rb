# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Verification::Issue do
  it 'omits metadata when absent and includes it when present' do
    issue = described_class.new(code: 'entry_hash_mismatch', message: 'entry hash differs',
                                chain_key: 'global', sequence: 1)
    completeness_issue = described_class.new(
      code: 'source_ledger_entry_missing', message: 'source rows are missing ledger entries',
      chain_key: nil, sequence: nil, metadata: { source_table: 'versions', missing_count: 2 }
    )

    expect(issue.to_h).to eq(
      code: 'entry_hash_mismatch', message: 'entry hash differs', chain_key: 'global', sequence: 1
    )
    expect(completeness_issue.to_h).to include(
      metadata: { source_table: 'versions', missing_count: 2 }
    )
  end
end
