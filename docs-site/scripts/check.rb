# frozen_string_literal: true

require "nokolexbor"
require "yaml"
require_relative "../config/documentation"

site = Pathname.new(__dir__).join("../../site")
docs = Documentation::ROOT
navigation = YAML.safe_load_file(File.join(__dir__, "../src/_data/navigation.yml"))
pages = navigation.flat_map { |entry| entry.fetch("pages", [entry]) }
paths = pages.map { |entry| Documentation.route(entry.fetch("path")) }
paths += docs.glob("**/*.md").map { |source| Documentation.route(source.relative_path_from(docs).to_s) }

missing = paths.uniq.reject { |route| site.join(route.delete_prefix("/"), "index.html").file? }
abort "Missing published documentation routes: #{missing.join(", ")}" unless missing.empty?

%w[api/openapi.v1.yaml llms.txt].each do |asset|
  abort "Published download is missing or changed: #{asset}" unless site.join(asset).binread == docs.join(asset).binread
end

{
  "index.html" => ".callout-important",
  "api/index.html" => "#canonical-addressing",
  "database-relationships/index.html" => "pre.mermaid",
  "search/index.html" => "#pagefind-search"
}.each do |file, selector|
  page = Nokolexbor::HTML(site.join(file).read)
  abort "Missing #{selector} in #{file}" unless page.at_css(selector)
end

home = Nokolexbor::HTML(site.join("index.html").read)
%w[/med-tracker/quick-start/ /med-tracker/api/].each do |url|
  abort "Missing navigation link #{url}" unless home.at_css(%(a[href="#{url}"]))
end

%w[_bridgetown/site.css _bridgetown/site.js pagefind/pagefind-entry.json sitemap.xml].each do |asset|
  abort "Missing site asset #{asset}" unless site.join(asset).size?
end
abort "Missing canonical homepage in the sitemap" unless site.join("sitemap.xml").read.include?("<loc>https://damacus.github.io/med-tracker/</loc>")
abort "The docs Taskfile was copied into the site" if site.join("Taskfile.yml").exist?

puts "Verified #{paths.uniq.size} documentation routes, navigation, downloads, anchors, and site assets."
