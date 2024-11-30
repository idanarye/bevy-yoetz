#!/bin/bash

(
    retrospective-crate-version-tagging detect \
        --crate-name bevy-yoetz \
        --changelog-path CHANGELOG.md \
        --tag-prefix v \
) | retrospective-crate-version-tagging create-releases
