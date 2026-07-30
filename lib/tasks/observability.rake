# frozen_string_literal: true

namespace :observability do
  desc 'Emit the safe deployed observability canary'
  task canary: :environment do
    Observability::DeployedCanary.run
  end
end
