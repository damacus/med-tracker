require 'rails_helper'

RSpec.describe Api::Sync do
  [
    [Api::Sync::InventoryOperation, 'medication_dosage_option', 'delete'],
    [Api::Sync::CareRecordOperation, 'person', 'delete'],
    [Api::Sync::AssignmentOperation, 'schedule', 'replace']
  ].each do |service_class, resource_type, action|
    it "rejects #{resource_type} #{action} before accessing a record" do
      service = service_class.new(authorization: nil, household: nil)
      expect do
        service.call(operation: { resource_type: resource_type, action: action, id: 'unavailable' })
      end.to raise_error(service_class::Error) { |error| expect(error.code).to eq('sync_operation_unsupported') }
    end
  end
end
