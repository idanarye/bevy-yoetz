# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `YoetzSuggestion` - trait and derive macro for describing behaviors.
- `YoetzAdvisor` component for representing the AI status and reading suggestion from systems.
- `YoetzPlugin` for cranking the state of `YoetzAdvisor` and adding/removing/updating other components when it changes.
