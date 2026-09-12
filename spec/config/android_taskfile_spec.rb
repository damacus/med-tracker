# frozen_string_literal: true

require 'rails_helper'

module AndroidTaskfile
end

RSpec.describe AndroidTaskfile do
  it 'keeps every APK variant and limits Gradle workers to one' do
    taskfile = Rails.root.join('mobile/android/Taskfile.yml').read

    expect(taskfile).to include('--max-workers=1')
    expect(taskfile).to include(
      ':phone:assembleDebug', ':phone:assembleStaging', ':phone:assembleRelease',
      ':wear:assembleDebug', ':wear:assembleStaging', ':wear:assembleRelease'
    )
  end
end
