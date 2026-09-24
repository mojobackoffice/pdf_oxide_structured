# frozen_string_literal: true

require "json"
require "pathname"
require_relative "pdf_oxide_structured/version"
# Load path, not this file's directory: Bundler installs the compiled
# extension outside the gem tree, while local compiles land in lib/.
require "pdf_oxide_structured/pdf_oxide_structured"

module PdfOxideStructured
  class Error < StandardError; end

  # Returns structured text spans for a PDF path or IO.
  #
  # `source` is a String path, a Pathname, or an IO that responds to `read`.
  # `page` is a 0-based index. When omitted, every page is returned.
  # The hash matches the pdf-extract CLI envelope: "file", "page_count",
  # and "pages" of "page_number" plus "spans".
  def self.extract(source, page: nil)
    case source
    when String, Pathname
      path = source.to_s
      parsed = JSON.parse(extract_path_json(path, page))
      parsed["file"] = path
      parsed
    else
      raise TypeError, "expected a String path, Pathname, or IO" unless source.respond_to?(:read)

      source.binmode if source.respond_to?(:binmode)
      parsed = JSON.parse(extract_bytes_json(source.read, page))
      parsed["file"] = source.respond_to?(:path) ? source.path : "<in-memory>"
      parsed
    end
  end
end
