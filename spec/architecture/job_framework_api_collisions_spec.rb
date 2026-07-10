# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Active Job framework API collisions' do
  it 'keeps private application job helpers away from the framework public API' do
    collisions = ApplicationJob.descendants.to_h do |job_class|
      [job_class.name, collisions_for(job_class)]
    end.compact_blank

    expect(collisions).to be_empty
  end

  it 'detects a private helper that shadows a framework accessor' do
    shadowing_job = Class.new(ApplicationJob) do
      private

      def scheduled_at; end
    end

    expect(collisions_for(shadowing_job)).to contain_exactly(:scheduled_at)
  end

  def collisions_for(job_class)
    job_class.private_instance_methods(false) & ActiveJob::Base.public_instance_methods
  end
end
