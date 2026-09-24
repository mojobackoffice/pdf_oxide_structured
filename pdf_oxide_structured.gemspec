# frozen_string_literal: true

require_relative "lib/pdf_oxide_structured/version"

Gem::Specification.new do |spec|
  spec.name = "pdf_oxide_structured"
  spec.version = PdfOxideStructured::VERSION
  spec.authors = ["MojoTools Team"]
  spec.summary = "Structured text spans from a PDF path or buffer"
  spec.description = "Rust extension that extracts pdf-extract text spans from a path, Pathname, or IO."
  spec.homepage = "https://github.com/mojobackoffice/mojo_tools"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2"

  spec.files = Dir[
    "lib/**/*",
    "src/**/*",
    "Cargo.toml",
    "Cargo.lock",
    "extconf.rb"
  ].reject { |path| path.end_with?(".bundle", ".so", ".dll") }
  spec.require_paths = ["lib"]
  spec.extensions = ["extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9.124"
end
