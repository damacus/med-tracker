# frozen_string_literal: true

require_relative "documentation"

Bridgetown.configure do
  init :willamette, bypass_style_guide: true
  init :external_sources do
    contents do
      pages "../docs"
    end
  end

  html_inspector_parser "nokolexbor"

  builder :DocumentationRoutes do
    def build
      permalink_placeholder :documentation_path do |resource|
        { raw_value: Documentation.route(resource.data.source_path).delete_prefix("/").delete_suffix("/") }
      end
    end
  end

  hook :pages, :post_read do |resource|
    resource.data.willamette = { heading_anchors: false, lazy_loaded_images: false }
    next unless resource.model.origin.original_path.to_s.start_with?("#{Documentation::ROOT}/")

    path = resource.relative_path.to_s.delete_prefix("/")
    next unless File.extname(path) == ".md"

    resource.data.layout = "documentation"
    resource.data.source_path = path
    resource.data.permalink = Documentation.route(path) == "/" ? "/" : "/:documentation_path/"
    resource.data.title ||= resource.content[/^# (.+)$/, 1] || File.basename(path, ".md")
  end

  hook :pages, :pre_render do |resource|
    next unless resource.data.source_path

    resource.content = Documentation.callouts(resource.content)
  end

  hook :site, :post_read do |site|
    site.collections["pages"].resources.reject! do |resource|
      path = resource.relative_path.to_s
      origin = resource.model.origin.original_path.to_s
      if origin.start_with?("#{Documentation::ROOT}/") && File.extname(path) != ".md"
        unless path == "Taskfile.yml"
          site.static_files << Bridgetown::StaticFile.new(
            site, Documentation::ROOT.to_s, File.dirname(path), File.basename(path)
          )
        end
        true
      else
        path == "search.erb" && resource.model.origin.is_a?(Bridgetown::Model::PluginOrigin)
      end
    end
    site.static_files.reject! do |file|
      Pathname.new(file.path).cleanpath == Documentation::ROOT.join("Taskfile.yml") ||
        file.relative_path.start_with?("/willamette/")
    end
  end
end
