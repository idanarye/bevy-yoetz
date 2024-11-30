# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## 0.3.0 - 2024-11-30
### Changed
- Upgrade Bevy to 0.15

## 0.2.0 - 2024-07-05
### Changed
- Upgrade Bevy to 0.14

### Added
- [**BREAKING**] `YoetzPlugin` can - and must - be configured to crank
  `YoetzAdvisor` at any schedule. This is a breaking change because
  `YoetzAdvisor::default()` is no longer available and `YoetzAdvisor::new()`
  must be used instead to specify the schedule.

## 0.1.0 - 2024-03-06
### Added
- `YoetzSuggestion` - trait and derive macro for describing behaviors.
- `YoetzAdvisor` component for representing the AI status and reading
  suggestion from systems.
- `YoetzPlugin` for cranking the state of `YoetzAdvisor` and
  adding/removing/updating other components when it changes.
