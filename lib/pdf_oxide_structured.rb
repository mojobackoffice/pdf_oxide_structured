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
    extract_json(source, page, :extract_path_json, :extract_bytes_json)
  end

  # Returns word-level text for a PDF path or IO.
  #
  # `source` and `page` match `extract`. Each page has "words" instead of
  # "spans". A word is a pdf_oxide TextBlock: "text", "bbox", "chars", and
  # font attributes.
  def self.extract_words(source, page: nil)
    extract_json(source, page, :extract_words_path_json, :extract_words_bytes_json)
  end

  # Parses a path or IO through the named native JSON methods and sets "file".
  def self.extract_json(source, page, path_method, bytes_method)
    case source
    when String, Pathname
      path = source.to_s
      parsed = JSON.parse(public_send(path_method, path, page))
      parsed["file"] = path
      parsed
    else
      raise TypeError, "expected a String path, Pathname, or IO" unless source.respond_to?(:read)

      source.binmode if source.respond_to?(:binmode)
      parsed = JSON.parse(public_send(bytes_method, source.read, page))
      parsed["file"] = source.respond_to?(:path) ? source.path : "<in-memory>"
      parsed
    end
  end
  private_class_method :extract_json
end
