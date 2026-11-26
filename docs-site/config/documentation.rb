# frozen_string_literal: true

require "cgi"
require "pathname"
require "uri"

module Documentation
  ROOT = Pathname.new(__dir__).join("../../docs").expand_path.freeze
  REPOSITORY = "https://github.com/damacus/med-tracker/blob/main"

  def self.route(path)
    path = path.delete_prefix("/")
    return "/#{path}" unless File.extname(path) == ".md"

    directory = path.sub(%r{(?:^|/)(?:index|README)\.md$}, "")
    return directory.empty? ? "/" : "/#{directory}/" if directory != path

    "/#{path.delete_suffix(".md")}/"
  end

  def self.callouts(markdown)
    markdown.gsub(/^!!! ([a-z]+) "([^"\n]+)"\n((?:^ {4}[^\n]*\n?)+)/) do
      kind = Regexp.last_match(1)
      title = CGI.escapeHTML(Regexp.last_match(2))
      body = Regexp.last_match(3).gsub(/^ {4}/, "")
      %(<aside class="callout callout-#{kind}" markdown="1">\n\n**#{title}**\n\n#{body}\n</aside>\n)
    end
  end

  def self.slug(text)
    text.unicode_normalize(:nfkd).encode("ASCII", invalid: :replace, undef: :replace, replace: "")
      .gsub(/[^\w\s-]/, "").strip.downcase.gsub(/[-\s]+/, "-")
  end

  def self.render(html, path, base_path)
    document = Nokolexbor::DocumentFragment.parse(html)
    used_ids = {}
    document.css("h1, h2, h3, h4, h5, h6").each do |heading|
      original = heading["id"]
      next unless original

      id = slug(heading.text)
      id = "#{id}_1" while used_ids[id] || id.empty?
      used_ids[id] = true
      heading["id"] = id
      heading.add_child(%(<a class="heading-link" href="##{id}" aria-label="Link to this section">¶</a>))
    end

    document.css("a[href], img[src], source[src]").each do |element|
      attribute = element.name == "a" ? "href" : "src"
      element[attribute] = destination(element[attribute], path, base_path)
    end

    document.css("pre > code.language-mermaid").each do |diagram|
      diagram.parent.replace(%(<pre class="mermaid">#{CGI.escapeHTML(diagram.text)}</pre>))
    end

    document.to_html
  end

  def self.destination(url, path, base_path)
    return url if url.match?(%r{\A(?:[a-z][a-z\d+.-]*:|//|#|\?|\z)}i)
    return url if url.start_with?("#{base_path}/")

    location, suffix = url.split(/(?=[?#])/, 2)
    target = if location.start_with?("/")
               ROOT.join(location.delete_prefix("/")).cleanpath
             else
               ROOT.join(File.dirname(path), URI::DEFAULT_PARSER.unescape(location)).cleanpath
             end
    relative = target.relative_path_from(ROOT).to_s
    if relative.start_with?("../")
      repo_path = target.relative_path_from(ROOT.parent).to_s
      return "#{REPOSITORY}/#{encode_path(repo_path)}#{suffix}"
    end

    "#{base_path}#{encode_path(route(relative))}#{suffix}"
  end

  def self.encode_path(path)
    URI::DEFAULT_PARSER.escape(path)
  end
end
