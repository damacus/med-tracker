require 'rails_helper'

RSpec.describe NhsDmdImportReconciliationJob do
  it 'fails active imports unchanged for at least thirty minutes' do
    import = NhsDmdImport.create!(uploaded_filename: 'stalled.zip', status: :importing)
    import.update_column(:updated_at, 30.minutes.ago)

    described_class.perform_now

    expect(import.reload).to have_attributes(
      status: 'failed',
      error_message: 'Import interrupted because the worker stopped reporting progress.'
    )
    expect(import.log).to include('Import interrupted because the worker stopped reporting progress.')
  end

  it 'preserves recently updated active imports' do
    import = NhsDmdImport.create!(uploaded_filename: 'recent.zip', status: :counting)

    described_class.perform_now

    expect(import.reload).to be_counting
  end

  it 'runs every five minutes in production' do
    schedule = YAML.safe_load_file(Rails.root.join('config/recurring.yml')).dig(
      'production', 'reconcile_nhs_dmd_imports'
    )

    expect(schedule).to include(
      'class' => described_class.name, 'schedule' => 'every 5 minutes', 'queue' => 'default'
    )
  end
end
