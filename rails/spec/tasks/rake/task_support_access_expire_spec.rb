# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task_name) { 'support_access:expire' }
  let(:task) { described_class[task_name] }

  before do
    Rails.application.load_tasks unless described_class.task_defined?(task_name)
    task.reenable
  end

  it 'prints sanitized operational evidence for processed natural expiries' do
    allow(ExpireSupportAccessSessionsJob).to receive(:perform_now).and_return(2)

    expect { task.invoke }.to output(
      "{\"event_type\":\"support_access_session.expired\",\"outcome\":\"success\",\"processed_count\":2}\n"
    ).to_stdout
  end
end
