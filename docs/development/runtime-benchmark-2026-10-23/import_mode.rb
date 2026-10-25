raise 'This initializer is only for the isolated local benchmark' unless ENV['BENCHMARK_LOCAL_ONLY'] == '1'

Rails.application.config.to_prepare do
  NhsDmdImportJob.queue_adapter = :inline if ENV.fetch('BENCHMARK_IMPORT_MODE') == 'inline'
end
