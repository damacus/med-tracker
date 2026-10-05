# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ActiveStorage::Blob do
  it 'does not expose the direct upload blob creation endpoint' do
    expect(post: '/rails/active_storage/direct_uploads').not_to be_routable
  end

  it 'does not expose the disk direct upload write endpoint' do
    expect(put: '/rails/active_storage/disk/test-token').not_to be_routable
  end
end
