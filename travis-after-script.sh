#!/usr/bin/env bash

if [ "$TRAVIS_PULL_REQUEST" != "false" ]; then
    # Bench variable
    echo "Benchmarking PR #$TRAVIS_PULL_REQUEST..." && \
    cargo bench > benches/PR_$TRAVIS_PULL_REQUEST && \
    # Bench master
    echo "Checking out $TRAVIS_BRANCH..." && \
    git remote set-branches origin $TRAVIS_BRANCH && \
    git fetch origin $TRAVIS_BRANCH && \
    git checkout $TRAVIS_BRANCH && \
    echo "Benchmarking $TRAVIS_BRANCH" && \
    cargo bench > benches/$TRAVIS_BRANCH && \
    echo "Performance comparison:" && \
    cd benches && \
    cargo benchcmp $TRAVIS_BRANCH PR_$TRAVIS_PULL_REQUEST;
fi
