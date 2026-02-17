# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task_name) { 'med_tracker:bootstrap_admin' }
  let(:task) { described_class[task_name] }

  before do
    Rails.application.load_tasks unless described_class.task_defined?(task_name)
    task.reenable
  end

  around do |example|
    previous_values = {
      'ADMIN_EMAIL' => ENV.fetch('ADMIN_EMAIL', nil),
      'ADMIN_PASSWORD' => ENV.fetch('ADMIN_PASSWORD', nil),
      'ADMIN_NAME' => ENV.fetch('ADMIN_NAME', nil),
      'ADMIN_DOB' => ENV.fetch('ADMIN_DOB', nil)
    }

    example.run
  ensure
    previous_values.each { |key, value| ENV[key] = value }
  end

  def set_admin_env(email: nil, password: nil, name: nil, dob: nil)
    ENV['ADMIN_EMAIL'] = email
    ENV['ADMIN_PASSWORD'] = password
    ENV['ADMIN_NAME'] = name
    ENV['ADMIN_DOB'] = dob
  end

  def expect_bootstrap_service_called
    expect(Admin::BootstrapService).to have_received(:call).with(
      email: 'bootstrap.admin@example.com',
      password: 'securepassword123',
      name: 'Bootstrap Admin',
      date_of_birth: '1980-02-01'
    )
  end

  it 'invokes bootstrap service with environment variables' do
    set_admin_env(
      email: 'bootstrap.admin@example.com',
      password: 'securepassword123',
      name: 'Bootstrap Admin',
      dob: '1980-02-01'
    )

    user = instance_double(User, email_address: 'bootstrap.admin@example.com')
    result = instance_double(Admin::BootstrapService::Result, success?: true, error: nil, user: user)

    allow(Admin::BootstrapService).to receive(:call).and_return(result)

    expect do
      task.invoke
      expect_bootstrap_service_called
    end.to output(/created|success/i).to_stdout
  end

  it 'fails fast when required environment variables are missing' do
    set_admin_env

    allow(Admin::BootstrapService).to receive(:call)

    expect do
      task.invoke
      expect(Admin::BootstrapService).not_to have_received(:call)
    end.to output(/missing required environment variables/i).to_stdout
  end
end
