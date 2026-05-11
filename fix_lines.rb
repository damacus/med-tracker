lines = File.readlines('spec/system/global_search_spec.rb')
File.open('spec/system/global_search_spec.rb', 'w') do |f|
  lines.each do |line|
    if line.include?('page.execute_script') && line.include?('KeyboardEvent')
      f.puts "    page.execute_script("
      f.puts "      'window.dispatchEvent(new KeyboardEvent(\"keydown\", { key: \"k\", ctrlKey: true, bubbles: true }))'"
      f.puts "    )"
    else
      f.puts line
    end
  end
end
